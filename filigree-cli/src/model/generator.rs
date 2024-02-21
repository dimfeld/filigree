use std::{borrow::Cow, ops::Deref, path::PathBuf};

use convert_case::{Case, Casing};
use error_stack::{Report, ResultExt};
use itertools::Itertools;
use rayon::prelude::*;
use serde_json::json;

use super::{
    field::{Access, FilterableType, ModelField, ModelFieldReference, ReferentialAction, SqlType},
    Model,
};
use crate::{
    config::Config,
    migrations::SingleMigration,
    model::field::SortableType,
    templates::{ModelRustTemplates, ModelSqlTemplates, Renderer},
    Error, ModelMap, RenderedFile, RenderedFileLocation,
};

pub struct ModelGenerator<'a> {
    pub model: Model,
    pub model_map: &'a ModelMap,
    pub(super) renderer: &'a Renderer<'a>,
    pub config: &'a Config,
    context: Option<tera::Context>,
}

impl<'a> ModelGenerator<'a> {
    pub fn new(
        config: &'a Config,
        renderer: &'a Renderer<'a>,
        model_map: &'a ModelMap,
        model: Model,
    ) -> Result<Self, Error> {
        let mut s = Self {
            config,
            model_map,
            model,
            renderer,
            context: None,
        };

        s.create_template_context()?;

        Ok(s)
    }

    pub fn template_context(&self) -> &tera::Context {
        self.context.as_ref().unwrap()
    }

    pub fn fixed_migrations() -> (Vec<SingleMigration<'static>>, Vec<SingleMigration<'static>>) {
        let before_up = vec![SingleMigration {
            name: "delete_log".to_string(),
            model: None,
            up: Cow::from(include_str!("../../sql/delete_log.up.sql")),
            down: Cow::from(include_str!("../../sql/delete_log.down.sql")),
        }];

        let after_up = vec![
            SingleMigration {
                name: "user_info".to_string(),
                model: None,
                up: Cow::from(include_str!("../../sql/user_info.up.sql")),
                down: Cow::from(include_str!("../../sql/user_info.down.sql")),
            },
            SingleMigration {
                name: "permissions".to_string(),
                model: None,
                up: Cow::from(include_str!("../../sql/create_permissions.up.sql")),
                down: Cow::from(include_str!("../../sql/create_permissions.down.sql")),
            },
            SingleMigration {
                name: "object_permissions".to_string(),
                model: None,
                up: Cow::from(include_str!("../../sql/create_object_permissions.up.sql")),
                down: Cow::from(include_str!("../../sql/create_object_permissions.down.sql")),
            },
        ];

        (before_up, after_up)
    }

    pub fn render_up_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.renderer
            .render(
                &PathBuf::new(),
                "model/migrate_up.sql.tera",
                crate::RenderedFileLocation::Api,
                self.template_context(),
            )
            .map(|f| f.contents)
    }

    pub fn render_down_migration(&self) -> Result<Vec<u8>, Report<Error>> {
        self.renderer
            .render(
                &PathBuf::new(),
                "model/migrate_down.sql.tera",
                crate::RenderedFileLocation::Api,
                self.template_context(),
            )
            .map(|f| f.contents)
    }

    pub fn render_model_directory(&self) -> Result<Vec<RenderedFile>, Report<Error>> {
        let base_path = PathBuf::from("src/models").join(self.model.module_name());

        let skip_files = [
            "model/main_mod.rs.tera",
            "model/sql_macros.tera",
            "model/migrate_up.sql.tera",
            "model/migrate_down.sql.tera",
            "model/select_base.sql.tera",
        ];

        let files = ModelSqlTemplates::iter()
            .chain(ModelRustTemplates::iter())
            .filter(|f| !skip_files.contains(&f.as_ref()))
            .collect::<Vec<_>>();

        let output = files
            .into_par_iter()
            .map(|file| {
                let filename = file
                    .strip_prefix("model/")
                    .unwrap()
                    .strip_suffix(".tera")
                    .unwrap();
                let path = base_path.join(filename);
                self.renderer
                    .render_with_full_path(
                        path,
                        &file,
                        RenderedFileLocation::Api,
                        &self.template_context(),
                    )
                    .attach_printable_lazy(|| format!("Model {}", self.model.name))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(output)
    }

    pub fn all_fields(&self) -> Result<impl Iterator<Item = Cow<ModelField>>, Error> {
        let fields = self
            .standard_fields()?
            .map(|field| Cow::Owned(field))
            .chain(self.fields.iter().map(|field| Cow::Borrowed(field)))
            .chain(self.reference_fields().map(|field| Cow::Owned(field)));

        Ok(fields)
    }

    pub fn write_payload_struct_fields(
        &self,
    ) -> Result<impl Iterator<Item = Cow<ModelField>>, Error> {
        Ok(self
            .all_fields()?
            .filter(|f| f.owner_access.can_write() && !f.never_read))
    }

    fn create_template_context(&mut self) -> Result<(), Error> {
        let sql_dialect = Config::default_sql_dialect();
        let full_default_sort_field = self.default_sort_field.as_deref().unwrap_or("-updated_at");
        let default_sort_field = if full_default_sort_field.starts_with('-') {
            &full_default_sort_field[1..]
        } else {
            full_default_sort_field
        };

        {
            let default_field = self.all_fields()?.find(|f| f.name == default_sort_field);
            if let Some(default_field) = default_field {
                if default_field.sortable == SortableType::None {
                    panic!(
                        "Model {}: Default sort field {default_sort_field} is not sortable",
                        self.name
                    );
                }
            } else {
                panic!(
                    "Model {}, Default sort field {} does not exist in model {}",
                    self.name, default_sort_field, self.name
                );
            }
        }

        let predefined_object_id =
            &["role", "user", "organization"].contains(&self.module_name().as_str());

        let fields = self
            .all_fields()?
            .map(|field| field.template_context())
            .collect::<Vec<_>>();

        let join_primary_keys = if let Some(model_names) = self.joins.as_ref() {
            let model1 = self.model_map.get(&model_names.0, "join")?;
            let model2 = self.model_map.get(&model_names.1, "join")?;

            let model1_id_field_name = model1.foreign_key_id_field_name();
            let model2_id_field_name = model2.foreign_key_id_field_name();

            format!("PRIMARY KEY({model1_id_field_name}, {model2_id_field_name})")
        } else {
            String::new()
        };

        let extra_create_table_sql = [&join_primary_keys, &self.extra_create_table_sql]
            .iter()
            .filter(|s| !s.is_empty())
            .join(",\n");

        let base_dir = PathBuf::from("src/models").join(self.module_name());

        let json_value = json!({
            "dir": base_dir,
            "module_name": &self.model.module_name(),
            "model_name": self.model.name,
            "sql_dialect": sql_dialect,
            "name": self.name,
            "plural": self.plural(),
            "table": self.table(),
            "indexes": self.indexes,
            "global": self.global,
            "fields": fields,
            "owner_permission": format!("{}::owner", self.name),
            "read_permission": format!("{}::read", self.name),
            "write_permission": format!("{}::write", self.name),
            "extra_sql": self.extra_sql,
            "extra_create_table_sql": extra_create_table_sql,
            "pagination": self.pagination,
            "full_default_sort_field": full_default_sort_field,
            "default_sort_field": default_sort_field,
            "id_type": self.object_id_type(),
            "id_prefix": self.id_prefix(),
            "predefined_object_id": predefined_object_id,
            "url_path": self.plural().to_lowercase(),
            "has_any_endpoints": self.endpoints.any_enabled(),
            "endpoints": self.endpoints.per_endpoint(),
            "auth_scope": self.auth_scope.unwrap_or(self.config.default_auth_scope),
        });

        let mut context = tera::Context::from_value(json_value).unwrap();
        self.add_rust_structs_to_context(&mut context)?;

        self.context = Some(context);
        Ok(())
    }

    /// The fields that apply to every object
    fn standard_fields(&self) -> Result<impl Iterator<Item = ModelField>, Error> {
        let org_field = if self.global {
            None
        } else {
            let org_id_nullable = self.name == "User";
            let org_id_foreign_key = self.name != "User";

            Some(ModelField {
                name: "organization_id".to_string(),
                typ: SqlType::Uuid,
                rust_type: Some("crate::models::organization::OrganizationId".to_string()),
                nullable: org_id_nullable,
                unique: false,
                indexed: true,
                sortable: SortableType::None,
                filterable: FilterableType::None,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                default_sql: String::new(),
                default_rust: String::new(),
                never_read: false,
                fixed: true,
                previous_name: None,
                references: org_id_foreign_key.then(|| {
                    ModelFieldReference::new(
                        "organizations",
                        "id",
                        Some(ReferentialAction::Cascade),
                    )
                }),
            })
        };

        let id_fields = if let Some(model_names) = self.joins.as_ref() {
            let model1 = self.model_map.get(&model_names.0, "join")?;
            let model2 = self.model_map.get(&model_names.1, "join")?;

            fn join_id_field(model: &Model) -> ModelField {
                ModelField {
                    name: model.foreign_key_id_field_name(),
                    typ: SqlType::Uuid,
                    rust_type: Some(model.object_id_type()),
                    nullable: false,
                    unique: false,
                    indexed: true,
                    filterable: FilterableType::Exact,
                    sortable: SortableType::None,
                    extra_sql_modifiers: String::new(),
                    user_access: Access::Read,
                    owner_access: Access::Read,
                    references: Some(ModelFieldReference::new(
                        model.table(),
                        "id",
                        Some(ReferentialAction::Cascade),
                    )),
                    default_sql: String::new(),
                    default_rust: String::new(),
                    never_read: false,
                    fixed: true,
                    previous_name: None,
                }
            }

            [Some(join_id_field(model1)), Some(join_id_field(model2))]
        } else {
            [
                Some(ModelField {
                    name: "id".to_string(),
                    typ: SqlType::Uuid,
                    rust_type: Some(self.object_id_type()),
                    nullable: false,
                    unique: false,
                    indexed: false,
                    filterable: FilterableType::Exact,
                    sortable: SortableType::None,
                    extra_sql_modifiers: "primary key".to_string(),
                    user_access: Access::Read,
                    owner_access: Access::Read,
                    references: None,
                    default_sql: String::new(),
                    default_rust: String::new(),
                    never_read: false,
                    fixed: true,
                    previous_name: None,
                }),
                None,
            ]
        };

        let other_fields = [
            org_field,
            Some(ModelField {
                name: "updated_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                indexed: false,
                filterable: FilterableType::Range,
                sortable: SortableType::DefaultDescending,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                references: None,
                default_sql: "now()".to_string(),
                default_rust: String::new(),
                never_read: false,
                fixed: true,
                previous_name: None,
            }),
            Some(ModelField {
                name: "created_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                nullable: false,
                unique: false,
                indexed: false,
                filterable: FilterableType::Range,
                sortable: SortableType::DefaultDescending,
                extra_sql_modifiers: String::new(),
                user_access: Access::Read,
                owner_access: Access::Read,
                references: None,
                default_sql: "now()".to_string(),
                default_rust: String::new(),
                never_read: false,
                fixed: true,
                previous_name: None,
            }),
        ]
        .into_iter();

        Ok(id_fields.into_iter().chain(other_fields).flatten())
    }

    fn reference_fields(&self) -> impl Iterator<Item = ModelField> {
        let belongs_to = self.belongs_to.as_ref().map(|belongs_to| {
            let id_field_name = format!("{}_id", belongs_to.to_case(Case::Snake));
            ModelField {
                name: id_field_name,
                typ: SqlType::Uuid,
                rust_type: Some(format!("{belongs_to}Id")),
                nullable: false,
                unique: false,
                indexed: true,
                filterable: FilterableType::None,
                sortable: super::field::SortableType::None,
                extra_sql_modifiers: String::new(),
                user_access: Access::Write,
                owner_access: Access::Write,
                references: Some(ModelFieldReference {
                    table: "organizations".to_string(),
                    field: "id".to_string(),
                    on_delete: Some(ReferentialAction::Cascade),
                    on_update: None,
                    deferrable: None,
                    populate_on_list: false,
                    populate_on_get: false,
                }),
                default_sql: String::new(),
                default_rust: String::new(),
                never_read: false,
                fixed: false,
                previous_name: None,
            }
        });

        [belongs_to].into_iter().flatten()
    }
}

impl Deref for ModelGenerator<'_> {
    type Target = Model;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}
