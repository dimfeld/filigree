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

impl Role {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> RoleId {
        <RoleId as Default>::default().into()
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

    pub fn default_name() -> String {
        <String as Default>::default().into()
    }

    pub fn default_description() -> Option<String> {
        None
    }
}

impl Default for Role {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            name: Self::default_name(),
            description: Self::default_description(),
            _permission: ObjectPermission::Owner,
        }
    }
}

#[derive(Deserialize, Debug, Clone, sqlx::FromRow)]
#[cfg_attr(test, derive(Serialize))]
pub struct RoleCreatePayloadAndUpdatePayload {
    pub name: String,
    pub description: Option<String>,
}

pub type RoleCreatePayload = RoleCreatePayloadAndUpdatePayload;

pub type RoleUpdatePayload = RoleCreatePayloadAndUpdatePayload;

impl RoleCreatePayloadAndUpdatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_name() -> String {
        <String as Default>::default().into()
    }

    pub fn default_description() -> Option<String> {
        None
    }
}

impl Default for RoleCreatePayloadAndUpdatePayload {
    fn default() -> Self {
        Self {
            name: Self::default_name(),
            description: Self::default_description(),
        }
    }
}

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
