use std::{borrow::Cow, collections::HashSet, ops::Deref, path::PathBuf};

use convert_case::{Case, Casing};
use error_stack::{Report, ResultExt};
use itertools::Itertools;
use rayon::prelude::*;
use serde_json::json;

use super::{
    field::{
        Access, FilterableType, ModelField, ModelFieldReference, ReferencePopulation,
        ReferentialAction, SqlType,
    },
    Endpoints, HasModel, Model,
};
use crate::{
    config::Config,
    migrations::SingleMigration,
    model::{field::SortableType, ReferenceFetchType},
    templates::{ModelRustTemplates, ModelSqlTemplates, Renderer},
    Error, GeneratorMap, ModelMap, RenderedFile, RenderedFileLocation,
};

pub struct ChildField<'a> {
    pub field: ModelField,
    pub model: &'a Model,
    pub many: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ReadOperation {
    Get,
    List,
}

pub struct ModelGenerator<'a> {
    pub model: Model,
    pub model_map: &'a ModelMap,
    pub(super) renderer: &'a Renderer<'a>,
    pub config: &'a Config,
    children: Vec<HasModel>,
    context: Option<tera::Context>,
}

impl<'a> ModelGenerator<'a> {
    pub fn new(
        config: &'a Config,
        renderer: &'a Renderer<'a>,
        model_map: &'a ModelMap,
        model: Model,
    ) -> Result<Self, Error> {
        let file_has = model.files.iter().map(|f| f.has_for_parent(&model));
        let children = model
            .has
            .iter()
            .map(|c| c.clone())
            .chain(file_has)
            .collect();
        Ok(Self {
            config,
            model_map,
            model,
            renderer,
            children,
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

        let populate_queries = if self.children.is_empty() {
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
            .chain(self.belongs_to_field()?.map(|field| Cow::Owned(field)));

        Ok(fields)
    }

    /// The fields that go into the update and create structures
    pub fn write_payload_struct_fields(
        &self,
        for_update: bool,
    ) -> Result<impl Iterator<Item = Cow<ModelField>>, Error> {
        // The ID field is only used for child models, but we just add it always, make it optional,
        // and ignore it in the other cases.
        let mut id_field = self.id_field();
        id_field.nullable = true;

        Ok(std::iter::once(Cow::Owned(id_field))
            .chain(
                self.all_fields()?
                    .filter(|f| f.owner_access.can_write() && !f.never_read),
            )
            .chain(
                self.write_payload_child_fields(for_update)?
                    .map(|f| Cow::Owned(f.field)),
            ))
    }

    /// Initialize the template context. This should be called immediately after all the generators
    /// are created but before any templates are rendered.
    pub fn create_template_context(
        &self,
        generators: &GeneratorMap,
    ) -> Result<tera::Context, Error> {
        let mut imports = HashSet::new();
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

        let mut fields = self
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

        let children = self.children.iter()
            .map(|has| {
                let child_model = self.model_map.get(&has.model, &self.model.name, "has")?;
                let child_generator = generators.get(&has.model, &self.model.name, "has")?;

                imports.insert(child_model.qualified_struct_name());
                imports.insert(format!("{}CreatePayload", child_model.qualified_struct_name()));
                imports.insert(format!("{}UpdatePayload", child_model.qualified_struct_name()));
                imports.insert(child_model.qualified_object_id_type());

                let get_sql_field_name = has.field_name.clone().unwrap_or_else(|| {
                    Self::child_model_field_name(&child_model, has.populate_on_get, has.many)
                });

                let list_sql_field_name = has.field_name.clone().unwrap_or_else(|| {
                    Self::child_model_field_name(&child_model, has.populate_on_list, has.many)
                });

                let get_field_type =
                    Self::child_model_field_type(&child_model, has.populate_on_get, has.many, "");
                let list_field_type =
                    Self::child_model_field_type(&child_model, has.populate_on_list, has.many, "");
                let exc = if has.many { "!" } else { "" };

                let url_path = if has.many {
                    child_model.plural().as_ref().to_case(Case::Snake)
                } else {
                    child_model.name.to_case(Case::Snake)
                };

                // Used in tests
                let possible_child_field_names = vec![
                    Self::child_model_field_name(&child_model, ReferenceFetchType::Id, false),
                    Self::child_model_field_name(&child_model, ReferenceFetchType::Id, true),
                    Self::child_model_field_name(&child_model, ReferenceFetchType::Data, false),
                    Self::child_model_field_name(&child_model, ReferenceFetchType::Data, true),
                ];

                let result = json!({
                    "relationship": has,
                    "get_field_type": get_field_type,
                    "get_sql_field_name": get_sql_field_name,
                    "full_get_sql_field_name": format!("{get_sql_field_name}{exc}: {get_field_type}"),
                    "list_field_type": list_field_type,
                    "list_sql_field_name": list_sql_field_name,
                    "write_payload_field_name": has.update_with_parent.then(|| has.rust_child_field_name(&child_model)),
                    "struct_base": child_model.struct_name(),
                    "insertable": has.update_with_parent,
                    "module": child_model.module_name(),
                    "possible_child_field_names": possible_child_field_names,
                    "object_id": child_model.object_id_type(),
                    "fields": child_generator.all_fields()?.map(|f| f.template_context()).collect::<Vec<_>>(),
                    "table": child_model.table(),
                    "url_path": url_path,
                    "parent_field": self.model.foreign_key_id_field_name(),
                    "file_upload": child_model.file_for.as_ref().map(|f| f.1.template_context()),
                });

                Ok::<_, Error>(result)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let references = self
            .reference_fields()?
            .map(|(id_field, populate, ref_model, field)| {
                let gen = generators.get(&ref_model.name, &self.model.name, "references")?;
                let full_name = field.qualified_sql_field_name();

                imports.insert(gen.model.qualified_struct_name());
                imports.insert(gen.model.qualified_object_id_type());

                Ok(json!({
                    "name": field.name,
                    "full_name": full_name,
                    "id_field": id_field,
                    "on_get": populate.on_get,
                    "on_list": populate.on_list,
                    "fields": gen.all_fields()?.map(|f| f.template_context()).collect::<Vec<_>>(),
                    "table": ref_model.table(),
                }))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        if let Some(b) = &self.belongs_to {
            let model = self
                .model_map
                .get(b.model(), &self.model.name, "belongs_to")?;
            imports.insert(model.qualified_object_id_type());
        }

        let parent_model_name = self.belongs_to.as_ref().map(|b| b.model());
        let belongs_to_field = self
            .belongs_to_field()?
            .next()
            .map(|f| f.template_context());

        let can_populate_get = self.virtual_fields(ReadOperation::Get)?.next().is_some();
        let can_populate_list = self.virtual_fields(ReadOperation::List)?.next().is_some();

        if let Some(b) = &belongs_to_field {
            let belongs_to_name = &b["sql_name"];
            for f in fields.iter_mut() {
                let is_belongs_to = f["name"].as_str().unwrap_or_default() == belongs_to_name;
                f["owner_write_non_parent"] =
                    json!(!is_belongs_to && f["owner_write"].as_bool().unwrap_or(false));
            }
        }

        let imports = imports
            .into_iter()
            .map(|i| format!("use {i};"))
            .sorted()
            .join("\n");

        let create_payload_fields = self
            .write_payload_child_fields(false)?
            .map(|f| {
                let mut context = f.field.template_context();
                context["many"] = f.many.into();
                context["module"] = f.model.module_name().into();
                context
            })
            .collect::<Vec<_>>();
        let update_payload_fields = self
            .write_payload_child_fields(false)?
            .map(|f| {
                let mut context = f.field.template_context();
                context["many"] = f.many.into();
                context["module"] = f.model.module_name().into();
                context
            })
            .collect::<Vec<_>>();

        let endpoints = if belongs_to_field.is_none() {
            &self.endpoints
        } else {
            // Right now we don't generate any endpoints for child models. They can only be
            // accessed through the endpoints for themselves on the parent model.
            &Endpoints::All(false)
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
            "create_payload_fields": create_payload_fields,
            "update_payload_fields": update_payload_fields,
            "imports": imports,
            "belongs_to_field": belongs_to_field,
            "can_populate_get": can_populate_get,
            "can_populate_list": can_populate_list,
            "children": children,
            "reference_populations": references,
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
            "url_path": self.plural().as_ref().to_case(Case::Snake),
            "has_any_endpoints": endpoints.any_enabled(),
            "endpoints": endpoints.per_endpoint(),
            "auth_scope": self.auth_scope.unwrap_or(self.config.default_auth_scope),
            "parent_model_name": parent_model_name,
            "file_for": self.file_for.as_ref().map(|f| f.0.as_str()),
            "file_upload": self.file_for.as_ref().map(|f| f.1.template_context()),
        });

        let mut context = tera::Context::from_value(json_value).unwrap();
        self.add_rust_structs_to_context(&mut context)?;

        Ok(context)
    }

    fn id_field(&self) -> ModelField {
        ModelField {
            name: "id".to_string(),
            typ: SqlType::Uuid,
            rust_type: Some(self.object_id_type()),
            zod_type: None,
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
        }
    }

    /// The fields that apply to every object
    fn standard_fields(&self) -> Result<impl Iterator<Item = ModelField>, Error> {
        let org_field = if self.global {
            None
        } else {
            let locked_to_single_org = self.name != "User";

            Some(ModelField {
                name: "organization_id".to_string(),
                typ: SqlType::Uuid,
                rust_type: Some("crate::models::organization::OrganizationId".to_string()),
                zod_type: None,
                nullable: !locked_to_single_org,
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
                references: locked_to_single_org.then(|| {
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
                    zod_type: None,
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
            [Some(self.id_field()), None]
        };

        let other_fields = [
            org_field,
            Some(ModelField {
                name: "updated_at".to_string(),
                typ: SqlType::Timestamp,
                rust_type: None,
                zod_type: None,
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
                zod_type: None,
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

    fn belongs_to_field(&self) -> Result<impl Iterator<Item = ModelField>, Error> {
        let belongs_to = self
            .belongs_to
            .as_ref()
            .map(|belongs_to| {
                let model =
                    self.model_map
                        .get(belongs_to.model(), &self.model.name, "belongs_to")?;

                // See if the parent model links back to this one, and if `many` is set or not.
                let single_child = model
                    .has
                    .iter()
                    .find(|has| has.model == self.model.name)
                    .map(|has| !has.many)
                    .unwrap_or(false)
                    || (self
                        .model
                        .file_for
                        .as_ref()
                        .map(|(parent_model, f)| parent_model == &model.name && f.many)
                        .unwrap_or(false));

                Ok::<_, Error>(ModelField {
                    name: model.foreign_key_id_field_name(),
                    typ: SqlType::Uuid,
                    rust_type: Some(model.object_id_type()),
                    zod_type: None,
                    nullable: belongs_to.optional(),
                    unique: single_child,
                    indexed: belongs_to.indexed(),
                    filterable: FilterableType::Exact,
                    sortable: super::field::SortableType::None,
                    extra_sql_modifiers: String::new(),
                    user_access: Access::ReadWrite,
                    owner_access: Access::ReadWrite,
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

    pub fn child_model_field_name(
        model: &Model,
        fetch_type: ReferenceFetchType,
        many: bool,
    ) -> String {
        match (fetch_type, many) {
            (ReferenceFetchType::None, _) => String::new(),
            (ReferenceFetchType::Id, false) => {
                format!("{}_id", model.name.to_case(Case::Snake))
            }
            (ReferenceFetchType::Id, true) => {
                format!("{}_ids", model.name.to_case(Case::Snake))
            }
            (ReferenceFetchType::Data, false) => model.name.to_case(Case::Snake),
            (ReferenceFetchType::Data, true) => model.plural().to_case(Case::Snake),
        }
    }

    pub fn child_model_field_type(
        model: &Model,
        fetch_type: ReferenceFetchType,
        many: bool,
        suffix: &str,
    ) -> String {
        match (fetch_type, many) {
            (ReferenceFetchType::None, _) => String::new(),
            (ReferenceFetchType::Id, false) => model.object_id_type(),
            (ReferenceFetchType::Id, true) => format!("Vec<{}>", model.object_id_type()),
            (ReferenceFetchType::Data, false) => format!("{}{suffix}", model.struct_name()),
            (ReferenceFetchType::Data, true) => format!("Vec<{}{suffix}>", model.struct_name()),
        }
    }

    pub fn child_model_zod_field_type(
        model: &Model,
        fetch_type: ReferenceFetchType,
        many: bool,
        suffix: &str,
    ) -> String {
        match (fetch_type, many) {
            (ReferenceFetchType::None, _) => String::new(),
            (ReferenceFetchType::Id, false) => "z.string().uuid()".to_string(),
            (ReferenceFetchType::Id, true) => "z.string().uuid().array()".to_string(),
            (ReferenceFetchType::Data, false) => format!("{}{suffix}Schema", model.struct_name()),
            (ReferenceFetchType::Data, true) => {
                format!("{}{suffix}Schema.array()", model.struct_name())
            }
        }
    }

    /// Fields generated in some SQL queries, such as when populating child models, but which are
    /// not present in the base table.
    /// These fields are not included in `all_fields`.
    pub fn virtual_fields(
        &self,
        read_operation: ReadOperation,
    ) -> Result<impl Iterator<Item = ModelField>, Error> {
        let base_field = ModelField {
            name: String::new(),
            typ: SqlType::Uuid,
            rust_type: None,
            zod_type: None,
            nullable: false,
            unique: false,
            extra_sql_modifiers: String::new(),
            user_access: Access::ReadWrite,
            owner_access: Access::ReadWrite,
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

        let file_has = self
            .files
            .iter()
            .map(|f| f.has_for_parent(self))
            .collect::<Vec<_>>();

        let has_fields = self
            .has
            .iter()
            .chain(file_has.iter())
            .map(|has| {
                let populate_type = match read_operation {
                    ReadOperation::Get => has.populate_on_get,
                    ReadOperation::List => has.populate_on_list,
                };
                let model = self.model_map.get(&has.model, &self.model.name, "has")?;

                let name = has.field_name.clone().unwrap_or_else(|| {
                    Self::child_model_field_name(&model, populate_type, has.many)
                });
                let rust_type = Self::child_model_field_type(&model, populate_type, has.many, "");
                let ts_type = Self::child_model_zod_field_type(&model, populate_type, has.many, "");

                if rust_type.is_empty() {
                    return Ok(None);
                }

                let field = ModelField {
                    name,
                    rust_type: Some(rust_type),
                    zod_type: Some(ts_type),
                    nullable: !has.many,
                    ..base_field.clone()
                };

                Ok::<_, Error>(Some(field))
            })
            .filter_map(|f| f.transpose())
            .collect::<Result<Vec<_>, Error>>()?;

        let reference_fields = self
            .reference_fields()?
            .filter_map(|(_, populate, _, field)| {
                match (read_operation, populate.on_get, populate.on_list) {
                    (ReadOperation::Get, true, _) => Some(field),
                    (ReadOperation::List, _, true) => Some(field),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();

        Ok(has_fields.into_iter().chain(reference_fields))
    }

    fn reference_fields(
        &self,
    ) -> Result<impl Iterator<Item = (String, &ReferencePopulation, &Model, ModelField)>, Error>
    {
        let base_field = ModelField {
            name: String::new(),
            typ: SqlType::Uuid,
            rust_type: None,
            zod_type: None,
            nullable: false,
            unique: false,
            extra_sql_modifiers: String::new(),
            user_access: Access::ReadWrite,
            owner_access: Access::ReadWrite,
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

        let fields = self
            .model
            .fields
            .iter()
            .map(|f| {
                let Some(populate) = f.references.as_ref().and_then(|r| r.populate.as_ref()) else {
                    return Ok(None);
                };

                if !populate.on_get && !populate.on_list {
                    return Ok(None);
                }

                let model =
                    self.model_map
                        .get(&populate.model, &self.model.name, "references.populate")?;

                let field_name = populate.field_name.clone().unwrap_or_else(|| {
                    Self::child_model_field_name(&model, ReferenceFetchType::Data, false)
                });

                let field = ModelField {
                    name: field_name,
                    rust_type: Some(model.struct_name()),
                    nullable: f.nullable,
                    ..base_field.clone()
                };
                Ok(Some((f.name.clone(), populate, model, field)))
            })
            .filter_map(|f| f.transpose())
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(fields.into_iter())
    }

    pub fn write_payload_child_field_type(
        has_model: &Model,
        has: &HasModel,
        for_update: bool,
    ) -> String {
        let suffix = if for_update {
            "UpdatePayload"
        } else {
            "CreatePayload"
        };

        let write_payload_type = match has.through {
            None => ReferenceFetchType::Data,
            Some(_) => ReferenceFetchType::Id,
        };

        let rust_type =
            Self::child_model_field_type(has_model, write_payload_type, has.many, suffix);

        // For the update payload, wrap a single child field in a double option so we can distinguish
        // between null (remove the child) vs. the member being absent (don't touch the
        // child).
        if for_update && !has.many {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        }
    }

    pub fn write_payload_child_fields(
        &self,
        for_update: bool,
    ) -> Result<impl Iterator<Item = ChildField>, Error> {
        let base_field = ModelField {
            name: String::new(),
            typ: SqlType::Uuid,
            rust_type: None,
            zod_type: None,
            nullable: false,
            unique: false,
            extra_sql_modifiers: String::new(),
            user_access: Access::ReadWrite,
            owner_access: Access::ReadWrite,
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
            .children
            .iter()
            .map(|has| {
                let has_model = self.model_map.get(&has.model, &self.model.name, "has")?;
                if !has.update_with_parent {
                    return Ok(None);
                }

                let rust_type = Self::write_payload_child_field_type(has_model, has, for_update);

                let model_field = ModelField {
                    name: has.rust_child_field_name(&has_model),
                    rust_type: Some(rust_type),
                    nullable: has.many,
                    ..base_field.clone()
                };

                let field = ChildField {
                    model: has_model,
                    many: has.many,
                    field: model_field,
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
