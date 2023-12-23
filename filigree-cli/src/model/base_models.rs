use super::{Access, Model};
use crate::model::{DeleteBehavior, ModelField, ModelFieldReference, SqlType};

fn simple_model_field(name: &str, typ: SqlType) -> ModelField {
    ModelField {
        name: name.to_string(),
        typ,
        rust_type: None,
        nullable: false,
        unique: false,
        filterable: super::field::FilterableType::None,
        sortable: super::field::SortableType::None,
        user_access: Access::Read,
        owner_access: Access::ReadWrite,
        default: String::new(),
        extra_sql_modifiers: String::new(),
        indexed: false,
        references: None,
    }
}

impl Model {
    /// Return models for the user, org, etc.
    pub fn create_default_models() -> Vec<Model> {
        vec![
            Model {
                name: "User".to_string(),
                plural: None,
                id_prefix: Some("usr".to_string()),
                endpoints: crate::model::Endpoints::All(true),
                auth_scope: Some(crate::model::ModelAuthScope::Model),
                global: false,
                indexes: vec![],
                default_sort_field: Some("name".to_string()),
                extra_create_table_sql: String::new(),
                pagination: Default::default(),
                fields: vec![
                    simple_model_field("name", SqlType::Text),
                    ModelField {
                        unique: true,
                        ..simple_model_field("email", SqlType::Text)
                    },
                    ModelField {
                        user_access: Access::None,
                        default: "false".into(),
                        ..simple_model_field("verified", SqlType::Boolean)
                    },
                ],
            },
            Model {
                name: "Organization".to_string(),
                plural: None,
                id_prefix: Some("tm".to_string()),
                global: true,
                endpoints: crate::model::Endpoints::All(false),
                indexes: vec![],
                default_sort_field: Some("name".to_string()),
                auth_scope: Some(crate::model::ModelAuthScope::Model),
                extra_create_table_sql: String::new(),
                pagination: Default::default(),
                fields: vec![
                    simple_model_field("name", SqlType::Text),
                    ModelField {
                        rust_type: Some("UserId".to_string()),
                        user_access: Access::None,
                        nullable: true,
                        references: Some(ModelFieldReference::new(
                            "users",
                            "id",
                            DeleteBehavior::SetNull,
                        )),
                        ..simple_model_field("owner", SqlType::Uuid)
                    },
                    ModelField {
                        user_access: Access::None,
                        owner_access: Access::None,
                        default: "true".into(),
                        ..simple_model_field("active", SqlType::Boolean)
                    },
                ],
            },
            Model {
                name: "Role".to_string(),
                plural: None,
                id_prefix: Some("rol".to_string()),
                global: false,
                endpoints: crate::model::Endpoints::All(true),
                indexes: vec![],
                default_sort_field: Some("name".to_string()),
                auth_scope: Some(crate::model::ModelAuthScope::Model),
                extra_create_table_sql: String::new(),
                pagination: Default::default(),
                fields: vec![
                    simple_model_field("name", SqlType::Text),
                    ModelField {
                        nullable: true,
                        user_access: Access::Read,
                        ..simple_model_field("description", SqlType::Text)
                    },
                ],
            },
        ]
    }
}
