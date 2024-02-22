use std::{borrow::Cow, ops::Deref, path::PathBuf};

use error_stack::{Report, ResultExt};
use itertools::Itertools;
use rayon::prelude::*;
use serde_json::json;

use super::{
    field::{Access, FilterableType, ModelField, ModelFieldReference, ReferentialAction, SqlType}, Model
};
use crate::{
    config::Config,
    migrations::SingleMigration,
    model::{field::SortableType, ReferenceFetchType},
    templates::{ModelRustTemplates, ModelSqlTemplates, Renderer},
    Error, GeneratorMap, ModelMap, RenderedFile, RenderedFileLocation,
};

pub enum ReadOperation {
    Get,
    List,
}

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
        Ok(Self {
            config,
            model_map,
            model,
            renderer,
            context: None,
        })
    }

    pub fn template_context(&self) -> &tera::Context {
        self.context
            .as_ref()
            .expect("called template_context before context was initialized")
    }

    pub fn set_template_context(&mut self, context: tera::Context) {
        self.context = Some(context);
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

        let mut populate_context = self.template_context().clone();
        populate_context.insert("populate_children", &true);

        let populate_queries = if self.model.has.is_empty() {
            vec![]
        } else {
            vec![
                ("model/select_one.sql.tera", "select_one_populated.sql"),
                ("model/list.sql.tera", "list_populated.sql"),
            ]
        }
        .into_iter()
        .map(|(infile, outfile)| {
            (
                Cow::Borrowed(infile),
                outfile.to_string(),
                &populate_context,
            )
        });

        let files = ModelSqlTemplates::iter()
            .chain(ModelRustTemplates::iter())
            .filter(|f| !skip_files.contains(&f.as_ref()))
            .map(|f| {
                let outfile = f
                    .strip_prefix("model/")
                    .unwrap()
                    .strip_suffix(".tera")
                    .unwrap()
                    .to_string();
                (f, outfile, self.template_context())
            })
            .chain(populate_queries)
            .collect::<Vec<_>>();

        let output = files
            .into_par_iter()
            .map(|(infile, outfile, ctx)| {
                let path = base_path.join(outfile);

                self.renderer
                    .render_with_full_path(path, &infile, RenderedFileLocation::Api, ctx)
                    .attach_printable_lazy(|| format!("Model {}", self.model.name))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(output)
    }

    /// All fields except fields generated when populating child models
    pub fn all_fields(&self) -> Result<impl Iterator<Item = Cow<ModelField>>, Error> {
        let fields = self
            .standard_fields()?
            .map(|field| Cow::Owned(field))
            .chain(self.fields.iter().map(|field| Cow::Borrowed(field)))
            .chain(self.reference_fields()?.map(|field| Cow::Owned(field)));

        Ok(fields)
    }

    pub fn write_payload_struct_fields(
        &self,
    ) -> Result<impl Iterator<Item = Cow<ModelField>>, Error> {
        Ok(self
            .all_fields()?
            .filter(|f| f.owner_access.can_write() && !f.never_read)
            .chain(self.write_payload_child_fields()?.map(Cow::Owned)))
    }

    /// Initialize the template context. This should be called immediately after all the generators
    /// are created but before any templates are rendered.
    pub fn create_template_context(
        &self,
        generators: &GeneratorMap,
    ) -> Result<tera::Context, Error> {
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
            let model1 = self
                .model_map
                .get(&model_names.0, &self.model.name, "join")?;
            let model2 = self
                .model_map
                .get(&model_names.1, &self.model.name, "join")?;

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

        let children = self
            .model
            .has
            .iter()
            .map(|has| {
                let child_model = self.model_map.get(&has.model, &self.model.name, "has")?;
                let child_generator = generators.get(&has.model, &self.model.name, "has")?;

                let get_sql_field_name = has.field_name.clone().unwrap_or_else(|| 
                    Self::child_model_field_name(&child_model, has.populate_on_get, has.many)
                );

                let list_sql_field_name = has.field_name.clone().unwrap_or_else(||
                    Self::child_model_field_name(&child_model, has.populate_on_list, has.many)
                );

                Ok::<_, Error>(json!({
                    "relationship": has,
                    "get_sql_field_name": get_sql_field_name,
                    "list_sql_field_name": list_sql_field_name,
                    "object_id": child_model.object_id_type(),
                    "plural": child_model.plural(),
                    "fields": child_generator.all_fields()?.map(|f| f.template_context()).collect::<Vec<_>>(),
                    "table": child_model.table(),
                    "parent_field": child_model.foreign_key_id_field_name(),
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let children = if children.is_empty() {
            json!(null)
        } else {
            json!(children)
        };

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
            "children": children,
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

        Ok(context)
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
            let model1 = self
                .model_map
                .get(&model_names.0, &self.model.name, "join")?;
            let model2 = self
                .model_map
                .get(&model_names.1, &self.model.name, "join")?;

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
                    references: Some(ModelFieldReference {
                        table: model.table(),
                        field: "id".to_string(),
                        on_delete: Some(ReferentialAction::Cascade),
                        on_update: None,
                        deferrable: Some(crate::model::field::Deferrable::InitiallyImmediate),
                        populate: None,
                    }),
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

    fn reference_fields(&self) -> Result<impl Iterator<Item = ModelField>, Error> {
        let belongs_to = self
            .belongs_to
            .as_ref()
            .map(|belongs_to| {
                let model =
                    self.model_map
                        .get(belongs_to.model(), &self.model.name, "belongs_to")?;
                Ok::<_, Error>(ModelField {
                    name: model.foreign_key_id_field_name(),
                    typ: SqlType::Uuid,
                    rust_type: Some(model.object_id_type()),
                    nullable: belongs_to.optional(),
                    unique: false,
                    indexed: belongs_to.indexed(),
                    filterable: FilterableType::Exact,
                    sortable: super::field::SortableType::None,
                    extra_sql_modifiers: String::new(),
                    user_access: Access::Write,
                    owner_access: Access::Write,
                    references: Some(ModelFieldReference {
                        table: model.table(),
                        field: "id".to_string(),
                        on_delete: Some(ReferentialAction::Cascade),
                        on_update: None,
                        deferrable: None,
                        populate: None,
                    }),
                    default_sql: String::new(),
                    default_rust: String::new(),
                    never_read: false,
                    fixed: false,
                    previous_name: None,
                })
            })
            .transpose()?;

        Ok([belongs_to].into_iter().flatten())
    }

    pub fn child_model_field_name(model: &Model, fetch_type: ReferenceFetchType, many: bool) -> String {
        match (fetch_type, many) {
            (ReferenceFetchType::None, _) => String::new(),
            (ReferenceFetchType::Id, true) => format!("{}_ids", model.plural()),
            (ReferenceFetchType::Id, false) => format!("{}_id", model.plural()),
            (ReferenceFetchType::Data, true) => model.plural().to_string(),
            (ReferenceFetchType::Data, false) => model.name.clone(),
        }
    }

    /// Fields generated in some SQL queries, such as when populating child models, but which are
    /// not present in the base table.
    /// This fields are not included in `all_fields`.
    pub fn virtual_fields(
        &self,
        read_operation: ReadOperation,
    ) -> Result<impl Iterator<Item = ModelField>, Error> {
        let base_field = ModelField {
            name: String::new(),
            typ: SqlType::Uuid,
            rust_type: None,
            nullable: false,
            unique: false,
            extra_sql_modifiers: String::new(),
            user_access: Access::Write,
            owner_access: Access::Write,
            default_sql: String::new(),
            default_rust: String::new(),
            indexed: false,
            filterable: FilterableType::None,
            sortable: SortableType::None,
            references: None,
            fixed: false,
            never_read: false,
            previous_name: None,
        };

        let has_fields = self
            .has
            .iter()
            .map(|has| {
                let populate_type = match read_operation {
                    ReadOperation::Get => has.populate_on_get,
                    ReadOperation::List => has.populate_on_list,
                };
                let model = self.model_map.get(&has.model, &self.model.name, "has")?;

                let name = has.field_name.clone();
                let field = match (populate_type, has.many) {
                    (ReferenceFetchType::None, _) => None,
                    (ReferenceFetchType::Id, true) => Some(ModelField {
                        name: name.unwrap_or_else(|| format!("{}_ids", model.table())),
                        rust_type: Some(format!("Vec<{}>", model.object_id_type())),
                        ..base_field.clone()
                    }),
                    (ReferenceFetchType::Id, false) => Some(ModelField {
                        name: name.unwrap_or_else(|| format!("{}_id", model.table())),
                        rust_type: Some(model.object_id_type()),
                        nullable: true,
                        ..base_field.clone()
                    }),
                    (ReferenceFetchType::Data, true) => Some(ModelField {
                        name: name.unwrap_or_else(|| model.plural().to_string()),
                        rust_type: Some(format!("Vec<{}>", model.struct_name())),
                        ..base_field.clone()
                    }),
                    (ReferenceFetchType::Data, false) => Some(ModelField {
                        name: name.unwrap_or_else(|| model.name.clone()),
                        rust_type: Some(model.struct_name()),
                        nullable: true,
                        ..base_field.clone()
                    }),
                };

                Ok::<_, Error>(field)
            })
            .filter_map(|f| f.transpose())
            .collect::<Result<Vec<_>, Error>>()?;

        let reference_fields = self
            .model
            .fields
            .iter()
            .map(|f| {
                let Some(populate) = f.references.as_ref().and_then(|r| r.populate.as_ref()) else {
                    return Ok(None);
                };

                let should_populate = match read_operation {
                    ReadOperation::Get => populate.on_get,
                    ReadOperation::List => populate.on_list,
                };

                if !should_populate {
                    return Ok(None);
                }

                let model =
                    self.model_map
                        .get(&populate.model, &self.model.name, "references.populate")?;

                let field_name = populate
                    .field_name
                    .clone()
                    .unwrap_or_else(|| format!("{}_data", f.name));

                Ok(Some(ModelField {
                    name: field_name,
                    rust_type: Some(model.struct_name()),
                    nullable: f.nullable,
                    ..base_field.clone()
                }))
            })
            .filter_map(|f| f.transpose())
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(has_fields.into_iter().chain(reference_fields.into_iter()))
    }

    pub fn write_payload_child_fields(&self) -> Result<impl Iterator<Item = ModelField>, Error> {
        let base_field = ModelField {
            name: String::new(),
            typ: SqlType::Uuid,
            rust_type: None,
            nullable: false,
            unique: false,
            extra_sql_modifiers: String::new(),
            user_access: Access::Write,
            owner_access: Access::Write,
            default_sql: String::new(),
            default_rust: String::new(),
            indexed: false,
            filterable: FilterableType::None,
            sortable: SortableType::None,
            references: None,
            fixed: false,
            never_read: false,
            previous_name: None,
        };

        let has_fields = self
            .has
            .iter()
            .map(|has| {
                let has_model = self.model_map.get(&has.model, &self.model.name, "has")?;
                if !has.update_with_parent {
                    return Ok(None);
                }

                let field = if has.many {
                    ModelField {
                        name: has
                            .field_name
                            .clone()
                            .unwrap_or_else(|| has_model.name.clone()),
                        rust_type: Some(has_model.struct_name()),
                        nullable: true,
                        ..base_field.clone()
                    }
                } else {
                    ModelField {
                        name: has
                            .field_name
                            .clone()
                            .unwrap_or_else(|| has_model.plural().to_string()),
                        rust_type: Some(format!("Vec<{}>", has_model.struct_name())),
                        nullable: false,
                        ..base_field.clone()
                    }
                };

                Ok(Some(field))
            })
            .filter_map(|f| f.transpose())
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(has_fields.into_iter())
    }
}

impl Deref for ModelGenerator<'_> {
    type Target = Model;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}
