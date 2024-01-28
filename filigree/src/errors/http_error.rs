use std::{borrow::Cow, fmt::Debug, ops::Deref};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use error_stack::Report;
use serde::Serialize;
use tracing::{event, Level};

/// An error that can be returned from an HTTP endpoint
pub trait HttpError: ToString + std::fmt::Debug {
    /// The type of the error detail. Can be [()] if there is no detail for this error.
    type Detail: Serialize + Debug + Send + Sync + 'static;

    /// The status code that the error should return.
    fn status_code(&self) -> StatusCode;
    /// An error code that may provide additional information to clients on how to behave in
    /// response to the error.
    fn error_kind(&self) -> &'static str;

    /// Extra detail about this error
    fn error_detail(&self) -> Self::Detail;

    /// The status code and data for this error. Most implementors of this trait will not
    /// need to override the default implementation.
    fn response_tuple(&self) -> (StatusCode, ErrorResponseData<Self::Detail>) {
        (
            self.status_code(),
            ErrorResponseData::new(self.error_kind(), self.to_string(), self.error_detail()),
        )
    }

    /// Return a value to force the [ObfuscateErrorLayer] to obfuscate this error's response in production, even if
    /// it would not otherwise do so.
    fn obfuscate(&self) -> Option<ForceObfuscate> {
        None
    }

    /// Convert the error into a [Response]. Most implementors of this trait will not
    /// need to override the default implementation.
    fn to_response(&self) -> Response {
        let (code, json) = self.response_tuple();
        let mut response = (code, Json(json)).into_response();

        if let Some(obfuscate) = self.obfuscate() {
            response.extensions_mut().insert(obfuscate);
        }

        response
    }
}

/// Error kind codes to return to the client in an error response.
pub enum ErrorKind {
    ApiKeyFormat,
    /// A generic ErrorKind when obfuscating a bad request error
    BadRequest,
    Database,
    DatabaseInit,
    Disabled,
    EmailSendFailure,
    FailedPredicate,
    FetchOAuthUserDetails,
    IncorrectPassword,
    InvalidApiKey,
    InvalidHostHeader,
    InvalidToken,
    MissingPermission,
    NotFound,
    NotVerified,
    OAuthExchangeError,
    OAuthSessionExpired,
    OAuthSessionNotFound,
    OrderBy,
    PasswordConfirmMismatch,
    PasswordHasherError,
    ServerStart,
    SessionBackend,
    SessionBackendError,
    Shutdown,
    SignupDisabled,
    Unauthenticated,
    UserCreationError,
    UserNotFound,
}

impl ErrorKind {
    /// Return the string form of the error kind code
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiKeyFormat => "invalid_api_key",
            Self::BadRequest => "bad_request",
            Self::Database => "database",
            Self::DatabaseInit => "db_init",
            Self::Disabled => "disabled",
            Self::EmailSendFailure => "email_send_failure",
            Self::FailedPredicate => "failed_authz_condition",
            Self::FetchOAuthUserDetails => "fetch_oauth_user_details",
            Self::IncorrectPassword => "incorrect_password",
            Self::InvalidApiKey => "invalid_api_key",
            Self::InvalidHostHeader => "invalid_host_header",
            Self::InvalidToken => "invalid_token",
            Self::MissingPermission => "missing_permission",
            Self::NotFound => "not_found",
            Self::NotVerified => "not_verified",
            Self::OAuthExchangeError => "oauth_exchange_error",
            Self::OAuthSessionExpired => "oauth_session_expired",
            Self::OAuthSessionNotFound => "oauth_session_not_found",
            Self::OrderBy => "order_by",
            Self::PasswordConfirmMismatch => "password_mismatch",
            Self::PasswordHasherError => "password_hash_internal",
            Self::ServerStart => "server",
            Self::SessionBackend => "session_backend",
            Self::SessionBackendError => "session_backend_error",
            Self::Shutdown => "shutdown",
            Self::SignupDisabled => "signup_disabled",
            Self::Unauthenticated => "unauthenticated",
            Self::UserCreationError => "user_creation_error",
            Self::UserNotFound => "user_not_found",
        }
    }
}

impl Into<Cow<'static, str>> for ErrorKind {
    fn into(self) -> Cow<'static, str> {
        Cow::Borrowed(self.as_str())
    }
}

/// Force error obfuscation and customize the values returned to the user.
#[derive(Clone, Debug, Default)]
pub struct ForceObfuscate {
    /// The code to return in the error
    pub kind: Cow<'static, str>,
    /// The message to return to in the error
    pub message: Cow<'static, str>,
}

impl ForceObfuscate {
    /// Create a new ForceObfuscate
    pub fn new(kind: impl Into<Cow<'static, str>>, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }

    /// A generic "Unauthenticated" error to return when the details of an authentication failure
    /// should be obfuscated.
    pub fn unauthenticated() -> Self {
        Self::new(ErrorKind::Unauthenticated, "Unauthenticated")
    }
}

impl<T> HttpError for error_stack::Report<T>
where
    T: HttpError + Send + Sync + 'static,
{
    type Detail = String;

    fn status_code(&self) -> StatusCode {
        self.current_context().status_code()
    }

    fn error_kind(&self) -> &'static str {
        self.current_context().error_kind()
    }

    /// Send the entire report detail as the detail
    fn error_detail(&self) -> String {
        format!("{self:?}")
    }
}

/// A body to be returned in an error response
#[derive(Debug, Serialize)]
pub struct ErrorResponseData<T: Debug + Serialize> {
    error: ErrorDetails<T>,
}

/// An error code and additional details.
#[derive(Debug, Serialize)]
pub struct ErrorDetails<T: Debug + Serialize> {
    kind: Cow<'static, str>,
    message: Cow<'static, str>,
    details: T,
}

impl<T: Debug + Serialize> ErrorResponseData<T> {
    /// Create a new [ErrorResponseData] with the given error code and message.
    pub fn new(
        kind: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
        details: T,
    ) -> ErrorResponseData<T> {
        let ret = ErrorResponseData {
            error: ErrorDetails {
                kind: kind.into(),
                message: message.into(),
                details: details.into(),
            },
        };

        event!(Level::ERROR, kind=%ret.error.kind, message=%ret.error.message, details=?ret.error.details);

        ret
    }
}

/// Wraps an error_stack::Report and implements IntoResponse, allowing easy return of a Report<T>
/// from an Axum endpoint.
pub struct WrapReport<T: HttpError + Sync + Send + 'static>(error_stack::Report<T>);

impl<T: HttpError + Sync + Send + 'static> IntoResponse for WrapReport<T> {
    fn into_response(self) -> Response {
        self.0.to_response()
    }
}

impl<T: HttpError + Sync + Send + 'static> From<Report<T>> for WrapReport<T> {
    fn from(value: Report<T>) -> Self {
        WrapReport(value)
    }
}

impl<T: HttpError + std::error::Error + Sync + Send + 'static> From<T> for WrapReport<T> {
    fn from(value: T) -> Self {
        WrapReport(Report::from(value))
    }
}

impl<T: HttpError + Sync + Send + 'static> Deref for WrapReport<T> {
    type Target = error_stack::Report<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
