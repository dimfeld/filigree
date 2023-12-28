#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};

use super::ReportId;
use crate::models::organization::OrganizationId;

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct ReportCreatePayload {
    pub title: String,
    pub description: Option<String>,
    pub ui: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct Report {
    pub id: ReportId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub title: String,
    pub description: Option<String>,
    pub ui: serde_json::Value,
    pub _permission: ObjectPermission,
}

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct ReportUpdatePayload {
    pub title: String,
    pub description: Option<String>,
    pub ui: Option<serde_json::Value>,
}

impl Serialize for Report {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self._permission == ObjectPermission::Owner {
            let mut state = serializer.serialize_struct("Report", 8)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("organization_id", &self.organization_id)?;
            state.serialize_field("updated_at", &self.updated_at)?;
            state.serialize_field("created_at", &self.created_at)?;
            state.serialize_field("title", &self.title)?;
            state.serialize_field("description", &self.description)?;
            state.serialize_field("ui", &self.ui)?;
            state.serialize_field("_permission", &self._permission)?;
            state.end()
        } else {
            let mut state = serializer.serialize_struct("Report", 8)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("organization_id", &self.organization_id)?;
            state.serialize_field("updated_at", &self.updated_at)?;
            state.serialize_field("created_at", &self.created_at)?;
            state.serialize_field("title", &self.title)?;
            state.serialize_field("description", &self.description)?;
            state.serialize_field("ui", &self.ui)?;
            state.serialize_field("_permission", &self._permission)?;
            state.end()
        }
    }
}
