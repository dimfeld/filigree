#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use sqlx_transparent_json_decode::sqlx_json_decode;

use super::ReportId;
use crate::models::{
    organization::OrganizationId,
    report_section::{
        ReportSection, ReportSectionCreatePayload, ReportSectionId, ReportSectionUpdatePayload,
    },
};

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

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

sqlx_json_decode!(Report);

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

impl Serialize for Report {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
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

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]
#[cfg_attr(test, derive(Serialize))]
pub struct ReportCreatePayload {
    pub id: Option<ReportId>,
    pub title: String,
    pub description: Option<String>,
    pub ui: serde_json::Value,
    pub report_sections: Option<Vec<ReportSectionCreatePayload>>,
}

impl ReportCreatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> Option<ReportId> {
        None
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

    pub fn default_report_sections() -> Option<Vec<ReportSectionCreatePayload>> {
        None
    }
}

impl Default for ReportCreatePayload {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            title: Self::default_title(),
            description: Self::default_description(),
            ui: Self::default_ui(),
            report_sections: Self::default_report_sections(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct ReportPopulatedGet {
    pub id: ReportId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub title: String,
    pub description: Option<String>,
    pub ui: serde_json::Value,
    pub report_sections: Vec<ReportSection>,
    pub _permission: ObjectPermission,
}

impl ReportPopulatedGet {
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

    pub fn default_report_sections() -> Vec<ReportSection> {
        <Vec<ReportSection> as Default>::default().into()
    }
}

sqlx_json_decode!(ReportPopulatedGet);

impl Default for ReportPopulatedGet {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            title: Self::default_title(),
            description: Self::default_description(),
            ui: Self::default_ui(),
            report_sections: Self::default_report_sections(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for ReportPopulatedGet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ReportPopulatedGet", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("ui", &self.ui)?;
        state.serialize_field("report_sections", &self.report_sections)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct ReportPopulatedList {
    pub id: ReportId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub title: String,
    pub description: Option<String>,
    pub ui: serde_json::Value,
    pub report_section_ids: Vec<ReportSectionId>,
    pub _permission: ObjectPermission,
}

impl ReportPopulatedList {
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

    pub fn default_report_section_ids() -> Vec<ReportSectionId> {
        <Vec<ReportSectionId> as Default>::default().into()
    }
}

sqlx_json_decode!(ReportPopulatedList);

impl Default for ReportPopulatedList {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            title: Self::default_title(),
            description: Self::default_description(),
            ui: Self::default_ui(),
            report_section_ids: Self::default_report_section_ids(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for ReportPopulatedList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ReportPopulatedList", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("ui", &self.ui)?;
        state.serialize_field("report_section_ids", &self.report_section_ids)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]
#[cfg_attr(test, derive(Serialize))]
pub struct ReportUpdatePayload {
    pub id: Option<ReportId>,
    pub title: String,
    pub description: Option<String>,
    pub ui: Option<serde_json::Value>,
    pub report_sections: Option<Vec<ReportSectionUpdatePayload>>,
}

impl ReportUpdatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> Option<ReportId> {
        None
    }

    pub fn default_title() -> String {
        <String as Default>::default().into()
    }

    pub fn default_description() -> Option<String> {
        None
    }

    pub fn default_ui() -> Option<serde_json::Value> {
        None
    }

    pub fn default_report_sections() -> Option<Vec<ReportSectionUpdatePayload>> {
        None
    }
}

impl Default for ReportUpdatePayload {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            title: Self::default_title(),
            description: Self::default_description(),
            ui: Self::default_ui(),
            report_sections: Self::default_report_sections(),
        }
    }
}
