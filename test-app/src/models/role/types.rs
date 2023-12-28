#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};

use super::RoleId;
use crate::models::organization::OrganizationId;

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct Role {
    pub id: RoleId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub name: String,
    pub description: Option<String>,
    pub _permission: ObjectPermission,
}

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct RoleCreatePayloadAndUpdatePayload {
    pub name: String,
    pub description: Option<String>,
}

pub type RoleCreatePayload = RoleCreatePayloadAndUpdatePayload;

pub type RoleUpdatePayload = RoleCreatePayloadAndUpdatePayload;

impl Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self._permission == ObjectPermission::Owner {
            let mut state = serializer.serialize_struct("Role", 7)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("organization_id", &self.organization_id)?;
            state.serialize_field("updated_at", &self.updated_at)?;
            state.serialize_field("created_at", &self.created_at)?;
            state.serialize_field("name", &self.name)?;
            state.serialize_field("description", &self.description)?;
            state.serialize_field("_permission", &self._permission)?;
            state.end()
        } else {
            let mut state = serializer.serialize_struct("Role", 7)?;
            state.serialize_field("id", &self.id)?;
            state.serialize_field("organization_id", &self.organization_id)?;
            state.serialize_field("updated_at", &self.updated_at)?;
            state.serialize_field("created_at", &self.created_at)?;
            state.serialize_field("name", &self.name)?;
            state.serialize_field("description", &self.description)?;
            state.serialize_field("_permission", &self._permission)?;
            state.end()
        }
    }
}
