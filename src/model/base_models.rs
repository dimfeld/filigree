use crate::model::{ModelField, SqlType};

use super::{Access, Model};

fn simple_model_field(name: &str, typ: SqlType, nullable: bool) -> ModelField {
    ModelField {
        name: name.to_string(),
        typ,
        rust_type: None,
        nullable,
        unique: false,
        user_access: Access::ReadWrite,
        owner_access: Access::ReadWrite,
        default: String::new(),
        extra_sql_modifiers: String::new(),
        indexed: false,
    }
}

impl Model {
    /// Return models for the user, team, permissions, etc.
    pub fn create_default_models() -> Vec<Model> {
        vec![
            Model {
                name: "User".to_string(),
                id_prefix: "usr".to_string(),
                endpoints: true,
                global: false,
                indexes: vec![],
                fields: vec![
                    simple_model_field("name", SqlType::Text, false),
                    ModelField {
                        unique: true,
                        user_access: Access::Read,
                        ..simple_model_field("email", SqlType::Text, false)
                    },
                    ModelField {
                        unique: true,
                        user_access: Access::None,
                        ..simple_model_field("password", SqlType::Text, false)
                    },
                ],
            },
            Model {
                name: "Team".to_string(),
                id_prefix: "tm".to_string(),
                global: true,
                endpoints: false,
                indexes: vec![],
                fields: vec![
                    simple_model_field("name", SqlType::Text, false),
                    ModelField {
                        rust_type: Some("UserId".to_string()),
                        user_access: Access::None,
                        ..simple_model_field("owner", SqlType::Uuid, true)
                    },
                ],
            },
            Model {
                name: "Role".to_string(),
                id_prefix: "rl".to_string(),
                global: false,
                endpoints: true,
                indexes: vec![],
                fields: vec![
                    simple_model_field("name", SqlType::Text, false),
                    simple_model_field("description", SqlType::Text, true),
                ],
            },
        ]
    }
}
