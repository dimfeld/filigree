use super::{Access, Model};
use crate::{
    config::Config,
    model::{field::ReferentialAction, ModelField, ModelFieldReference, PerEndpoint, SqlType},
};

fn simple_model_field(name: &str, typ: SqlType) -> ModelField {
    ModelField {
        name: name.to_string(),
        label: None,
        description: None,
        typ,
        rust_type: None,
        zod_type: None,
        nullable: false,
        unique: false,
        globally_unique: false,
        filterable: super::field::FilterableType::None,
        sortable: super::field::SortableType::None,
        access: Access::ReadWrite,
        omit_in_list: false,
        default_sql: String::new(),
        default_rust: String::new(),
        extra_sql_modifiers: String::new(),
        indexed: false,
        references: None,
        never_read: false,
        fixed: false,
        previous_name: None,
    }
}

impl Model {
    /// Return models for the user, org, etc.
    pub fn create_default_models(config: &Config) -> Vec<Model> {
        let extend_config = config.extend.models.as_ref();
        let extra_user_fields = extend_config
            .and_then(|c| c.user.as_ref())
            .map(|m| m.fields.clone())
            .unwrap_or_default();
        let extra_role_fields = extend_config
            .and_then(|c| c.role.as_ref())
            .map(|m| m.fields.clone())
            .unwrap_or_default();
        let extra_organization_fields = extend_config
            .and_then(|c| c.organization.as_ref())
            .map(|m| m.fields.clone())
            .unwrap_or_default();

        let auth_id_type = if config.auth.string_ids() {
            SqlType::Text
        } else {
            SqlType::Uuid
        };

        vec![
            Model {
                name: "User".to_string(),
                plural: None,
                id_prefix: Some("usr".to_string()),
                standard_endpoints: crate::model::Endpoints::Only(PerEndpoint {
                    get: true,
                    list: true,
                    create: false,
                    update: true,
                    delete: true,
                }),
                endpoints: Vec::new(),
                auth_scope: Some(crate::model::ModelAuthScope::Model),
                global: false,
                indexes: vec![],
                index_created_at: false,
                index_updated_at: false,
                default_sort_field: Some("name".to_string()),
                extra_create_table_sql: String::new(),
                extra_sql: String::new(),
                pagination: Default::default(),
                files: Vec::new(),
                shared_types: Vec::new(),
                allow_id_in_create: false,
                joins: None,
                belongs_to: vec![],
                has: vec![],
                file_for: None,
                is_auth_model: true,
                schema: config.database.auth_schema().map(|s| s.to_string()),
                fields: [
                    ModelField {
                        sortable: super::field::SortableType::DefaultAscending,
                        ..simple_model_field("name", SqlType::Text)
                    },
                    ModelField {
                        access: Access::None,
                        nullable: true,
                        never_read: true,
                        ..simple_model_field("password_hash", SqlType::Text)
                    },
                    ModelField {
                        globally_unique: true,
                        nullable: true,
                        ..simple_model_field("email", SqlType::Text)
                    },
                    ModelField {
                        nullable: true,
                        ..simple_model_field("avatar_url", SqlType::Text)
                    },
                ]
                .into_iter()
                .chain(extra_user_fields.into_iter())
                .collect(),
            },
            Model {
                name: "Organization".to_string(),
                plural: None,
                id_prefix: Some("org".to_string()),
                global: true,
                standard_endpoints: crate::model::Endpoints::All(false),
                indexes: vec![],
                index_created_at: false,
                index_updated_at: false,
                default_sort_field: Some("name".to_string()),
                endpoints: Vec::new(),
                auth_scope: Some(crate::model::ModelAuthScope::Model),
                extra_create_table_sql: String::new(),
                extra_sql: String::new(),
                pagination: Default::default(),
                allow_id_in_create: false,
                files: Vec::new(),
                shared_types: Vec::new(),
                joins: None,
                belongs_to: vec![],
                has: vec![],
                file_for: None,
                is_auth_model: true,
                schema: config.database.auth_schema().map(|s| s.to_string()),
                fields: [
                    ModelField {
                        sortable: super::field::SortableType::DefaultAscending,
                        ..simple_model_field("name", SqlType::Text)
                    },
                    ModelField {
                        rust_type: Some("crate::models::user::UserId".to_string()),
                        nullable: true,
                        references: config.auth.builtin().then(|| {
                            ModelFieldReference::new("User", "id", Some(ReferentialAction::SetNull))
                                .with_deferrable(
                                    crate::model::field::Deferrable::InitiallyImmediate,
                                )
                        }),
                        ..simple_model_field("owner", auth_id_type)
                    },
                    ModelField {
                        rust_type: Some("crate::models::role::RoleId".to_string()),
                        nullable: true,
                        references: None,
                        ..simple_model_field("default_role", auth_id_type)
                    },
                    ModelField {
                        access: Access::None,
                        default_sql: "true".into(),
                        ..simple_model_field("active", SqlType::Boolean)
                    },
                ]
                .into_iter()
                .chain(extra_organization_fields.into_iter())
                .collect(),
            },
            Model {
                name: "Role".to_string(),
                plural: None,
                id_prefix: Some("rol".to_string()),
                global: false,
                standard_endpoints: crate::model::Endpoints::All(true),
                indexes: vec![],
                index_created_at: false,
                index_updated_at: false,
                default_sort_field: Some("name".to_string()),
                auth_scope: Some(crate::model::ModelAuthScope::Model),
                endpoints: Vec::new(),
                extra_create_table_sql: String::new(),
                extra_sql: String::new(),
                pagination: Default::default(),
                files: Vec::new(),
                shared_types: Vec::new(),
                allow_id_in_create: false,
                joins: None,
                belongs_to: vec![],
                has: vec![],
                file_for: None,
                is_auth_model: true,
                schema: config.database.auth_schema().map(|s| s.to_string()),
                fields: [
                    ModelField {
                        sortable: super::field::SortableType::DefaultAscending,
                        ..simple_model_field("name", SqlType::Text)
                    },
                    ModelField {
                        nullable: true,
                        ..simple_model_field("description", SqlType::Text)
                    },
                ]
                .into_iter()
                .chain(extra_role_fields.into_iter())
                .collect(),
            },
        ]
    }
}
