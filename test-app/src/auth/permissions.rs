use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::auth::Authed;

#[derive(Debug, Serialize)]
pub struct PermissionInfo {
    name: &'static str,
    description: &'static str,
    key: &'static str,
}

pub const PERMISSIONS: &[PermissionInfo] = &[
    PermissionInfo {
        name: "Read Comments",
        description: "List and read Comment objects",
        key: "Comment::read",
    },
    PermissionInfo {
        name: "Write Comments",
        description: "Write Comment objects",
        key: "Comment::write",
    },
    PermissionInfo {
        name: "Administer Comments",
        description: "Create and delete Comment objects",
        key: "Comment::owner",
    },
    PermissionInfo {
        name: "Read Users",
        description: "List and read User objects",
        key: "User::read",
    },
    PermissionInfo {
        name: "Write Users",
        description: "Write User objects",
        key: "User::write",
    },
    PermissionInfo {
        name: "Administer Users",
        description: "Create and delete User objects",
        key: "User::owner",
    },
    PermissionInfo {
        name: "Read Organizations",
        description: "List and read Organization objects",
        key: "Organization::read",
    },
    PermissionInfo {
        name: "Write Organizations",
        description: "Write Organization objects",
        key: "Organization::write",
    },
    PermissionInfo {
        name: "Administer Organizations",
        description: "Create and delete Organization objects",
        key: "Organization::owner",
    },
    PermissionInfo {
        name: "Read Polls",
        description: "List and read Poll objects",
        key: "Poll::read",
    },
    PermissionInfo {
        name: "Write Polls",
        description: "Write Poll objects",
        key: "Poll::write",
    },
    PermissionInfo {
        name: "Administer Polls",
        description: "Create and delete Poll objects",
        key: "Poll::owner",
    },
    PermissionInfo {
        name: "Read PostImages",
        description: "List and read PostImage objects",
        key: "PostImage::read",
    },
    PermissionInfo {
        name: "Write PostImages",
        description: "Write PostImage objects",
        key: "PostImage::write",
    },
    PermissionInfo {
        name: "Administer PostImages",
        description: "Create and delete PostImage objects",
        key: "PostImage::owner",
    },
    PermissionInfo {
        name: "Read Reactions",
        description: "List and read Reaction objects",
        key: "Reaction::read",
    },
    PermissionInfo {
        name: "Write Reactions",
        description: "Write Reaction objects",
        key: "Reaction::write",
    },
    PermissionInfo {
        name: "Administer Reactions",
        description: "Create and delete Reaction objects",
        key: "Reaction::owner",
    },
    PermissionInfo {
        name: "Read Posts",
        description: "List and read Post objects",
        key: "Post::read",
    },
    PermissionInfo {
        name: "Write Posts",
        description: "Write Post objects",
        key: "Post::write",
    },
    PermissionInfo {
        name: "Administer Posts",
        description: "Create and delete Post objects",
        key: "Post::owner",
    },
    PermissionInfo {
        name: "Read ReportSections",
        description: "List and read ReportSection objects",
        key: "ReportSection::read",
    },
    PermissionInfo {
        name: "Write ReportSections",
        description: "Write ReportSection objects",
        key: "ReportSection::write",
    },
    PermissionInfo {
        name: "Administer ReportSections",
        description: "Create and delete ReportSection objects",
        key: "ReportSection::owner",
    },
    PermissionInfo {
        name: "Read Reports",
        description: "List and read Report objects",
        key: "Report::read",
    },
    PermissionInfo {
        name: "Write Reports",
        description: "Write Report objects",
        key: "Report::write",
    },
    PermissionInfo {
        name: "Administer Reports",
        description: "Create and delete Report objects",
        key: "Report::owner",
    },
    PermissionInfo {
        name: "Read Roles",
        description: "List and read Role objects",
        key: "Role::read",
    },
    PermissionInfo {
        name: "Write Roles",
        description: "Write Role objects",
        key: "Role::write",
    },
    PermissionInfo {
        name: "Administer Roles",
        description: "Create and delete Role objects",
        key: "Role::owner",
    },
];

pub async fn list_permissions(_authed: Authed) -> impl IntoResponse {
    Json(PERMISSIONS)
}
