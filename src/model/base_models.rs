use crate::model::{ModelField, SqlType};

use super::Model;

fn simple_model_field(name: &str, typ: SqlType, nullable: bool, public: bool) -> ModelField {
    ModelField {
        name: name.to_string(),
        typ,
        rust_type: None,
        nullable,
        unique: false,
        public,
        indexes: vec![],
        default: String::new(),
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
                fields: vec![ModelField {
                    default: "None".to_string().into(),
                    ..simple_model_field("name", SqlType::Text, false, true)
                }],
            },
            Model {
                name: "Team".to_string(),
                id_prefix: "tm".to_string(),
                endpoints: false,
                fields: vec![simple_model_field("name", SqlType::Text, false, true)],
            },
        ]
    }
}
