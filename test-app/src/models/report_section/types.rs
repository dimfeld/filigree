#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use sqlx_transparent_json_decode::sqlx_json_decode;

use super::ReportSectionId;
use crate::models::{organization::OrganizationId, report::ReportId};

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct ReportSection {
    pub id: ReportSectionId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub name: String,
    pub viz: String,
    pub options: serde_json::Value,
    pub report_id: ReportId,
    pub _permission: ObjectPermission,
}

pub type ReportSectionPopulatedGet = ReportSection;

pub type ReportSectionPopulatedList = ReportSection;

impl ReportSection {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> ReportSectionId {
        <ReportSectionId as Default>::default().into()
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

    pub fn default_viz() -> String {
        <String as Default>::default().into()
    }

    pub fn default_options() -> serde_json::Value {
        <serde_json::Value as Default>::default().into()
    }

    pub fn default_report_id() -> ReportId {
        <ReportId as Default>::default().into()
    }
}

sqlx_json_decode!(ReportSection);

impl Default for ReportSection {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            name: Self::default_name(),
            viz: Self::default_viz(),
            options: Self::default_options(),
            report_id: Self::default_report_id(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for ReportSection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ReportSection", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("viz", &self.viz)?;
        state.serialize_field("options", &self.options)?;
        state.serialize_field("report_id", &self.report_id)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]
#[cfg_attr(test, derive(Serialize))]
pub struct ReportSectionCreatePayloadAndUpdatePayload {
    pub id: Option<ReportSectionId>,
    pub name: String,
    pub viz: String,
    pub options: serde_json::Value,
    pub report_id: ReportId,
}

pub type ReportSectionCreatePayload = ReportSectionCreatePayloadAndUpdatePayload;

pub type ReportSectionUpdatePayload = ReportSectionCreatePayloadAndUpdatePayload;

impl ReportSectionCreatePayloadAndUpdatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> Option<ReportSectionId> {
        None
    }

    pub fn default_name() -> String {
        <String as Default>::default().into()
    }

    pub fn default_viz() -> String {
        <String as Default>::default().into()
    }

    pub fn default_options() -> serde_json::Value {
        <serde_json::Value as Default>::default().into()
    }

    pub fn default_report_id() -> ReportId {
        <ReportId as Default>::default().into()
    }
}

impl Default for ReportSectionCreatePayloadAndUpdatePayload {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            name: Self::default_name(),
            viz: Self::default_viz(),
            options: Self::default_options(),
            report_id: Self::default_report_id(),
        }
    }
}
