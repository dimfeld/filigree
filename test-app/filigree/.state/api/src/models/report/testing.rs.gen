use super::{ReportCreatePayload, ReportId, ReportUpdatePayload};
use crate::models::report_section::{
    ReportSection, ReportSectionCreatePayload, ReportSectionId, ReportSectionUpdatePayload,
};

/// Generate a ReportCreatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_create_payload(i: usize) -> ReportCreatePayload {
    ReportCreatePayload {
        id: None,
        title: format!("Test object {i}"),
        description: (i > 1).then(|| format!("Test object {i}")),
        ui: serde_json::json!({ "key": i }),

        report_section: match i {
            0 => None,
            1 => Some(vec![
                crate::models::report_section::testing::make_create_payload(i),
            ]),
            _ => Some(vec![
                crate::models::report_section::testing::make_create_payload(i),
                crate::models::report_section::testing::make_create_payload(i + 1),
            ]),
        },
    }
}

/// Generate a ReportUpdatePayload for testing.
/// Parameter `i` controls the value of some of the fields, just to make sure that the objects
/// don't all look identical.
pub fn make_update_payload(i: usize) -> ReportUpdatePayload {
    ReportUpdatePayload {
        id: None,
        title: format!("Test object {i}"),
        description: Some(format!("Test object {i}")),
        ui: Some(serde_json::json!({ "key": i })),

        report_section: match i {
            0 => None,
            1 => Some(vec![
                crate::models::report_section::testing::make_update_payload(i),
            ]),
            _ => Some(vec![
                crate::models::report_section::testing::make_update_payload(i),
                crate::models::report_section::testing::make_update_payload(i + 1),
            ]),
        },
    }
}
