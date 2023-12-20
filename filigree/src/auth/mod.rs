mod extractors;
/// Authentication middleware
pub mod middleware;
mod sessions;

use axum::response::IntoResponse;
pub use extractors::*;
use thiserror::Error;

use crate::{errors::HttpError, make_object_id};

make_object_id!(UserId, usr);
make_object_id!(OrganizationId, org);
make_object_id!(RoleId, rol);

/// An error related to authentication
#[derive(Debug, Error)]
pub enum AuthError {
    /// The user is not logged in
    #[error("Not authenticated")]
    Unauthenticated,
    /// The user is known, but requires verification before they can do most operations
    #[error("User is not verified")]
    NotVerified,
    /// The user or organization is inactive
    #[error("User or org is disabled")]
    Disabled,
}

impl HttpError for AuthError {
    fn status_code(&self) -> axum::http::StatusCode {
        match self {
            Self::Unauthenticated => axum::http::StatusCode::UNAUTHORIZED,
            Self::NotVerified | Self::Disabled => axum::http::StatusCode::FORBIDDEN,
        }
    }

    fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::NotVerified => "not_verified",
            Self::Disabled => "disabled",
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        self.to_response()
    }
}

// pub struct AuthUser {
//     pub id: UserId,
//     pub active: bool,
//     pub verified: bool,
// }

// pub struct AuthRole {
//     pub id: RoleId,
// }

// pub struct AuthOrganization {
//     pub id: OrganizationId,
//     pub active: bool,
// }

// pub struct AuthInfo {
//     pub user: AuthUser,
//     pub organization: AuthOrganization,
//     pub roles: Vec<AuthRole>,
//     pub all_permissions: Vec<String>,
// }

// impl AuthInfo {
//     pub fn actor_ids(&self) -> Vec<&Uuid> {
//         self.roles
//             .iter()
//             .map(|role| role.id.as_uuid())
//             .chain(std::iter::once(self.user.id.as_uuid()))
//             .collect()
//     }
// }

/// An object containing information about the current user.
pub trait AuthInfo: Clone + Send + Sync {
    /// Return Ok if the user is valid, or an [AuthError] if the user is not authenticated or
    /// authorized.
    fn check_valid(&self) -> Result<(), AuthError>;
    /// Check if the user, or any of its associated objects (roles, etc.) has a specific permission.
    fn has_permission(permission: &str) -> bool;
}

// TODO require permission middleware layer
// TODO predicate middleware layer
