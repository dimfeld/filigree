#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};

use super::OrganizationId;

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub name: String,
    pub owner: Option<crate::models::user::UserId>,
    pub active: bool,
    pub _permission: ObjectPermission,
}

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct OrganizationCreatePayloadAndUpdatePayload {
    pub name: String,
    pub owner: Option<crate::models::user::UserId>,
}

pub type OrganizationCreatePayload = OrganizationCreatePayloadAndUpdatePayload;

pub type OrganizationUpdatePayload = OrganizationCreatePayloadAndUpdatePayload;

impl Serialize for Organization {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self._permission == ObjectPermission::Owner {
            let mut state = serializer.serialize_struct("Organization", 6)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("updated_at", &self.updated_at)?;
            state.serialize_field("created_at", &self.created_at)?;
            state.serialize_field("name", &self.name)?;
            state.serialize_field("owner", &self.owner)?;
            state.serialize_field("_permission", &self._permission)?;
            state.end()
        } else {
            let mut state = serializer.serialize_struct("Organization", 5)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("updated_at", &self.updated_at)?;
            state.serialize_field("created_at", &self.created_at)?;
            state.serialize_field("name", &self.name)?;
            state.serialize_field("_permission", &self._permission)?;
            state.end()
        }
    }
}
