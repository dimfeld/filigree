#![allow(unused_imports, unused_variables, dead_code)]
use super::{ReportSectionCreatePayload, ReportSectionId, ReportSectionUpdatePayload};
use crate::models::report::ReportId;

/// Generate a ReportSectionCreatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_create_payload(i: usize) -> ReportSectionCreatePayload {
    ReportSectionCreatePayload {
        id: None,
        name: format!("Test object {i}"),
        viz: format!("Test object {i}"),
        options: serde_json::json!({ "key": i }),
        report_id: <ReportId as Default>::default(),
    }
}

/// Generate a ReportSectionUpdatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_update_payload(i: usize) -> ReportSectionUpdatePayload {
    ReportSectionUpdatePayload {
        id: None,
        name: format!("Test object {i}"),
        viz: format!("Test object {i}"),
        options: serde_json::json!({ "key": i }),
        report_id: <ReportId as Default>::default(),
    }
}
