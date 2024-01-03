#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};

use super::ReportId;
use crate::models::organization::OrganizationId;

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

impl Report {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> ReportId {
        <ReportId as Default>::default().into()
    }

    pub fn default_organization_id() -> crate::models::organization::OrganizationId {
        <crate::models::organization::OrganizationId as Default>::default().into()
    }

    pub fn default_updated_at() -> chrono::DateTime<chrono::Utc> {
        <chrono::DateTime<chrono::Utc> as Default>::default().into()
    }

    pub fn default_created_at() -> chrono::DateTime<chrono::Utc> {
        <chrono::DateTime<chrono::Utc> as Default>::default().into()
    }

    pub fn default_title() -> String {
        <String as Default>::default().into()
    }

    pub fn default_description() -> Option<String> {
        None
    }

    pub fn default_ui() -> serde_json::Value {
        <serde_json::Value as Default>::default().into()
    }
}

impl Default for Report {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            title: Self::default_title(),
            description: Self::default_description(),
            ui: Self::default_ui(),
            _permission: ObjectPermission::Owner,
        }
    }
}

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct ReportCreatePayload {
    pub title: String,
    pub description: Option<String>,
    pub ui: serde_json::Value,
}

impl ReportCreatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_title() -> String {
        <String as Default>::default().into()
    }

    pub fn default_description() -> Option<String> {
        None
    }

    pub fn default_ui() -> serde_json::Value {
        <serde_json::Value as Default>::default().into()
    }
}

impl Default for ReportCreatePayload {
    fn default() -> Self {
        Self {
            title: Self::default_title(),
            description: Self::default_description(),
            ui: Self::default_ui(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct ReportUpdatePayload {
    pub title: String,
    pub description: Option<String>,
    pub ui: Option<serde_json::Value>,
}

impl ReportUpdatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_title() -> String {
        <String as Default>::default().into()
    }

    pub fn default_description() -> Option<String> {
        None
    }

    pub fn default_ui() -> Option<serde_json::Value> {
        None
    }
}

impl Default for ReportUpdatePayload {
    fn default() -> Self {
        Self {
            title: Self::default_title(),
            description: Self::default_description(),
            ui: Self::default_ui(),
        }
    }
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
