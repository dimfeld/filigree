/// Functions for working with API keys
pub mod api_key;
mod check_middleware;
/// HTTP endpoints for authentication
pub mod endpoints;
mod extractors;
/// A Request extension for lazy lookup of user auth info
pub mod lookup;
/// Authentication middleware
pub mod middleware;
/// OAuth Functionalty
pub mod oauth;
/// Functions for generating and verifying password hashes
pub mod password;
/// Functionalty for passwordless email-based login.
pub mod passwordless_email_login;
mod sessions;

use std::{borrow::Cow, sync::Arc};

use async_trait::async_trait;
use axum::{http::StatusCode, response::IntoResponse};
pub use check_middleware::*;
use clap::ValueEnum;
pub use extractors::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use sessions::*;
use thiserror::Error;
use uuid::Uuid;

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
    /// Internal error hashing a password
    #[error("Error hashing password")]
    PasswordHasherError(String),
    /// Occurs when the API key is in the wrong format.
    #[error("API key format does not match")]
    ApiKeyFormat,
    /// The user is missing a permission requred for an operation
    #[error("Missing permission {0}")]
    MissingPermission(Cow<'static, str>),
    /// The [has_auth_predicate] middleware rejected a user
    #[error("Auth error: {0}")]
    FailedPredicate(Cow<'static, str>),
    /// Generic error to wrap errors from the session backend
    #[error("Session backend error")]
    SessionBackend,
    /// The email sending service returned an error
    #[error("Email send failure")]
    EmailSendFailure,
    /// Missing or expired token
    #[error("Missing or expired token")]
    InvalidToken,
    /// Password and confirmation value do not match when updating password
    #[error("Passwords do not match")]
    PasswordMismatch,
}

impl AuthError {
    /// Return true if this error is [AuthError::Unauthenticated]
    pub fn is_unauthenticated(&self) -> bool {
        matches!(self, Self::Unauthenticated)
    }
}

impl HttpError for AuthError {
    type Detail = ();

    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidApiKey | Self::InvalidToken | Self::Unauthenticated => {
                StatusCode::UNAUTHORIZED
            }
            Self::NotVerified
            | Self::Disabled
            | Self::MissingPermission(_)
            | Self::FailedPredicate(_) => StatusCode::FORBIDDEN,
            Self::ApiKeyFormat | Self::PasswordMismatch => StatusCode::BAD_REQUEST,
            Self::Db(_)
            | Self::EmailSendFailure
            | Self::PasswordHasherError(_)
            | Self::SessionBackend => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_detail(&self) -> Self::Detail {
        ()
    }

    fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::InvalidApiKey => "invalid_api_key",
            Self::InvalidToken => "invalid_token",
            Self::NotVerified => "not_verified",
            Self::Disabled => "disabled",
            Self::Db(_) => "db",
            Self::PasswordMismatch => "password_mismatch",
            Self::ApiKeyFormat => "invalid_api_key",
            Self::PasswordHasherError(_) => "password_hash_internal",
            Self::MissingPermission(_) => "missing_permission",
            Self::FailedPredicate(_) => "failed_authz_condition",
            Self::SessionBackend => "session_backend",
            Self::EmailSendFailure => "email_send_failure",
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        self.to_response()
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(value: sqlx::Error) -> Self {
        Self::Db(Arc::new(value))
    }
}

/// Queries to fetch relevant user information from the database given an API key or a session ID.
#[async_trait]
pub trait AuthQueries: Send + Sync {
    /// The type returned by the queries
    type AuthInfo: AuthInfo;

    /// Fetch the AuthInfo from an API key. If you used the filigree CLI scaffolding,
    /// this should be `include_str!("src/auth/fetch_api_key.sql")`
    async fn get_user_by_api_key(
        &self,
        api_key: Uuid,
        key_hash: Vec<u8>,
    ) -> Result<Option<Self::AuthInfo>, sqlx::Error>;
    /// Fetch the AuthInfo from a session key. If you used the filigree CLI scaffolding,
    /// this should run `include_str!("src/auth/fetch_session.sql")`
    async fn get_user_by_session_id(
        &self,
        session_key: &SessionKey,
    ) -> Result<Option<Self::AuthInfo>, sqlx::Error>;
}

/// An object containing information about the current user.
pub trait AuthInfo: 'static + Send + Sync {
    /// Return Ok if the user is valid, or an [AuthError] if the user is not authenticated or
    /// authorized.
    fn check_valid(&self) -> Result<(), AuthError>;
    /// Check if the user, or any of its associated objects (roles, etc.) has a specific permission.
    fn has_permission(&self, permission: &str) -> bool;

    /// Check that the user has a permission, and return an error if they do not.
    fn require_permission(&self, permission: &'static str) -> Result<(), AuthError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(AuthError::MissingPermission(permission.into()))
        }
    }
}

/// The permission level of an object
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema, sqlx::Type)]
#[sqlx(rename_all = "snake_case", type_name = "text")]
#[serde(rename_all = "snake_case")]
pub enum ObjectPermission {
    /// The object is read-only
    Read,
    /// The object can be written
    Write,
    /// The user has ownership-level permissions
    Owner,
}

/// The result of a login, with an optional place to redirect to
#[derive(Debug, Serialize)]
pub struct LoginResult {
    /// A message to show to the user
    pub message: Cow<'static, str>,
    /// Where to go next
    pub redirect_to: Option<String>,
}

/// Cross-origin Resource Sharing (CORS) configuration
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, ValueEnum)]
pub enum CorsSetting {
    /// Don't configure CORS at all, which prevents any cross-origin request from being accepted
    /// if nothing else in the request chain (e.g. a reverse proxy) sets the Access-Control headers.
    #[default]
    None,
    /// Allow cross-origin requests from any host in the `hosts` list
    AllowHostList,
    /// Allow all hosts to access /api routes. Cookies are still not permitted.
    AllowAll,
}
