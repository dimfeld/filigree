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
/// OAuth Functionality
pub mod oauth;
/// Functions for generating and verifying password hashes
pub mod password;
/// Functionalty for passwordless email-based login.
pub mod passwordless_email_login;
mod sessions;

use std::borrow::Cow;

use async_trait::async_trait;
use axum::{http::StatusCode, response::IntoResponse};
pub use check_middleware::*;
use clap::ValueEnum;
use error_stack::Report;
pub use extractors::*;
use http::request::Parts;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use sessions::*;
use thiserror::Error;
use uuid::Uuid;

pub use self::lookup::FallbackAnonymousUser;
use crate::{
    errors::{ErrorKind, ForceObfuscate, HttpError},
    make_object_id,
};

make_object_id!(UserId, usr);
make_object_id!(OrganizationId, org);
make_object_id!(RoleId, rol);

/// An error related to authentication
#[derive(Clone, Debug, Error)]
pub enum AuthError {
    /// The user is not logged in
    #[error("Not authenticated")]
    Unauthenticated,
    /// User was not found. This will be obfuscated to a generic "unauthenticated" error
    #[error("User not found")]
    UserNotFound,
    /// Password was incorrect. This will be obfuscated to a generic "unauthenticated" error
    #[error("Incorrect password")]
    IncorrectPassword,
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
    #[error("Database error")]
    Db,
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
    PasswordConfirmMismatch,
}

impl AuthError {
    /// Return true if this error is [AuthError::Unauthenticated] or a similar error that indicates
    /// a failed login
    pub fn is_unauthenticated(&self) -> bool {
        matches!(
            self,
            Self::Unauthenticated
                | Self::UserNotFound
                | Self::IncorrectPassword
                | Self::InvalidToken
        )
    }
}

impl HttpError for AuthError {
    type Detail = ();

    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidApiKey
            | Self::InvalidToken
            | Self::Unauthenticated
            | Self::UserNotFound
            | Self::IncorrectPassword => StatusCode::UNAUTHORIZED,
            Self::NotVerified
            | Self::Disabled
            | Self::MissingPermission(_)
            | Self::FailedPredicate(_) => StatusCode::FORBIDDEN,
            Self::ApiKeyFormat | Self::PasswordConfirmMismatch => StatusCode::BAD_REQUEST,
            Self::Db
            | Self::EmailSendFailure
            | Self::PasswordHasherError(_)
            | Self::SessionBackend => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_detail(&self) -> Self::Detail {
        ()
    }

    fn obfuscate(&self) -> Option<ForceObfuscate> {
        self.is_unauthenticated()
            .then(ForceObfuscate::unauthenticated)
    }

    fn error_kind(&self) -> &'static str {
        match self {
            Self::InvalidApiKey => ErrorKind::InvalidApiKey,
            Self::InvalidToken => ErrorKind::InvalidToken,
            Self::Unauthenticated => ErrorKind::Unauthenticated,
            Self::UserNotFound => ErrorKind::UserNotFound,
            Self::IncorrectPassword => ErrorKind::IncorrectPassword,
            Self::NotVerified => ErrorKind::NotVerified,
            Self::Disabled => ErrorKind::Disabled,
            Self::ApiKeyFormat => ErrorKind::ApiKeyFormat,
            Self::PasswordConfirmMismatch => ErrorKind::PasswordConfirmMismatch,
            Self::MissingPermission(_) => ErrorKind::MissingPermission,
            Self::FailedPredicate(_) => ErrorKind::FailedPredicate,
            Self::Db => ErrorKind::Database,
            Self::EmailSendFailure => ErrorKind::EmailSendFailure,
            Self::PasswordHasherError(_) => ErrorKind::PasswordHasherError,
            Self::SessionBackend => ErrorKind::SessionBackend,
        }
        .as_str()
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        self.to_response()
    }
}

/// The result of an [AuthQueries] function
pub enum UserFromRequestPartsValue<T: AuthInfo> {
    /// Found information for the credentials
    Found(T),
    /// Did not find any information for the provided credentials
    NotFound,
    /// This auth method is not implemented, and so the auth system should ignore it.
    /// This is often the same as `NotFound`, except for `get_user_from_request_parts`,
    /// in which it indicates that the auth system should try all the other authentication
    /// methods.
    NotImplemented,
}

impl<T: AuthInfo> UserFromRequestPartsValue<T> {
    /// Convert `Self::Found(T)` into `Some(T)` and other values to `None`
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Found(t) => Some(t),
            Self::NotFound | Self::NotImplemented => None,
        }
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
    ) -> Result<Option<Self::AuthInfo>, Report<AuthError>>;
    /// Fetch the AuthInfo from a session key. If you used the filigree CLI scaffolding,
    /// this should run `include_str!("src/auth/fetch_session.sql")`
    async fn get_user_by_session_id(
        &self,
        session_key: &SessionKey,
    ) -> Result<Option<Self::AuthInfo>, Report<AuthError>>;

    /// Create an [AuthInfo] object for an anonymous user.
    async fn anonymous_user(
        &self,
        user: UserId,
    ) -> Result<Option<Self::AuthInfo>, Report<AuthError>> {
        Ok(None)
    }

    /// Custom processing of the request to extract the user. This can be used to implement some
    /// other authentication scheme. If you implement this, the other methods will not be called.
    async fn get_user_from_request_parts(
        &self,
        parts: &Parts,
    ) -> Result<UserFromRequestPartsValue<Self::AuthInfo>, Report<AuthError>> {
        Ok(UserFromRequestPartsValue::NotImplemented)
    }
}

/// An object containing information about the current user.
pub trait AuthInfo: 'static + Send + Sync {
    /// Return Ok if the user is valid, or an [AuthError] if the user is not authenticated or
    /// authorized.
    fn check_valid(&self) -> Result<(), AuthError>;
    /// Return true if this user is the anonymous user.
    fn is_anonymous(&self) -> bool;
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

impl ObjectPermission {
    /// Create an ObjectPermission from a string, returning None if the value does not match.
    pub fn from_str_infallible(perm: &str) -> Option<ObjectPermission> {
        match perm {
            "owner" => Some(ObjectPermission::Owner),
            "read" => Some(ObjectPermission::Read),
            "write" => Some(ObjectPermission::Write),
            _ => None,
        }
    }

    /// Return true if the permission is Write or Owner
    pub fn can_write(&self) -> bool {
        matches!(self, ObjectPermission::Write | ObjectPermission::Owner)
    }

    /// Return an error if this [ObjectPermission] does not allow writing. The permission_name
    /// parameter is only used to return a better error message, and does not factor into the logic here.
    pub fn must_be_writable(&self, permission_name: &'static str) -> Result<(), AuthError> {
        if self.can_write() {
            Ok(())
        } else {
            Err(AuthError::MissingPermission(Cow::Borrowed(permission_name)))
        }
    }
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
