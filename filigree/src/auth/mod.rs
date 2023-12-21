mod extractors;
pub mod lookup;
/// Authentication middleware
pub mod middleware;
mod sessions;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{http::StatusCode, response::IntoResponse};
pub use extractors::*;
use sqlx::{postgres::PgRow, FromRow, PgConnection};
use thiserror::Error;
use uuid::Uuid;

use self::sessions::{SessionId, SessionKey};
use crate::{errors::HttpError, make_object_id};

make_object_id!(UserId, usr);
make_object_id!(OrganizationId, org);
make_object_id!(RoleId, rol);

/// An error related to authentication
#[derive(Clone, Debug, Error)]
pub enum AuthError {
    /// The user is not logged in
    #[error("Not authenticated")]
    Unauthenticated,
    /// An API key was provided but it does not exist or is inactive
    #[error("Invalid API Key")]
    InvalidApiKey,
    /// The user is known, but requires verification before they can do most operations
    #[error("User is not verified")]
    NotVerified,
    /// The user or organization is inactive
    #[error("User or org is disabled")]
    Disabled,
    /// The database returned an error
    #[error("Database error {0}")]
    Db(#[from] Arc<sqlx::Error>),
}

impl HttpError for AuthError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidApiKey | Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::NotVerified | Self::Disabled => StatusCode::FORBIDDEN,
            Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::InvalidApiKey => "invalid_api_key",
            Self::NotVerified => "not_verified",
            Self::Disabled => "disabled",
            Self::Db(_) => "db",
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

/// Queries to fetch relevant user information from the database given an API key or a session ID.
#[async_trait]
pub trait AuthQueries: Send + Sync {
    /// The type returned by the queries
    type AuthInfo: AuthInfo;

    /// Fetch the AuthInfo from an API key. If you used the filigree CLI scaffolding,
    /// this should be `include_str!("src/auth/fetch_api_key.sql")`
    async fn get_user_by_api_key(
        &self,
        api_key: &str,
    ) -> Result<Option<Self::AuthInfo>, sqlx::Error>;
    /// Fetch the AuthInfo from a session key. If you used the filigree CLI scaffolding,
    /// this should run `include_str!("src/auth/fetch_session.sql")`
    async fn get_user_by_session_id(
        &self,
        session_key: &SessionKey,
    ) -> Result<Option<Self::AuthInfo>, sqlx::Error>;
}

/// An object containing information about the current user.
pub trait AuthInfo: Clone + Send + Sync + Unpin + for<'db> FromRow<'db, PgRow> {
    /// Return Ok if the user is valid, or an [AuthError] if the user is not authenticated or
    /// authorized.
    fn check_valid(&self) -> Result<(), AuthError>;
    /// Check if the user, or any of its associated objects (roles, etc.) has a specific permission.
    fn has_permission(permission: &str) -> bool;
}

// TODO require permission middleware layer
// TODO predicate middleware layer
