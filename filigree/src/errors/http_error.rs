use std::{borrow::Cow, fmt::Debug, ops::Deref, sync::Arc};

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
        let (code, err) = self.response_tuple();
        event!(Level::ERROR, code=%code, kind=%err.error.kind, message=%err.error.message, details=?err.error.details);

        let form = err.form.clone();
        let mut response = (code, Json(err)).into_response();

        if let Some(mut obfuscate) = self.obfuscate() {
            // Attach form to the obfuscated data if present, since we want to pass it through.
            if obfuscate.form.is_none() {
                obfuscate = if let Some(form) = form {
                    obfuscate.with_form(form)
                } else {
                    obfuscate
                };
            }

            response.extensions_mut().insert(obfuscate);
        }

        response
    }
}

/// Error kind codes to return to the client in an error response.
pub enum ErrorKind {
    /// Invalid API key format
    ApiKeyFormat,
    /// A generic ErrorKind when obfuscating a bad request error
    BadRequest,
    /// Error communicating with the database
    Database,
    /// Error initializing the database connection
    DatabaseInit,
    /// User or organization is inactive
    Disabled,
    /// Error from the email sending service
    EmailSendFailure,
    /// A permissions predicate failed
    FailedPredicate,
    /// An OAuth login seemed to work, but fetching the user's details failed.
    FetchOAuthUserDetails,
    /// The password was incorrect. In production this should be obfuscated
    IncorrectPassword,
    /// The API key provided in a request was invalid or disabled
    InvalidApiKey,
    /// The Host header supplied in a request did not match an expected host
    InvalidHostHeader,
    /// The token provided in a reset request was invalid or expired
    InvalidToken,
    /// The requested operation requires a permission that the client does not have
    MissingPermission,
    /// The requested object was not found
    NotFound,
    /// The user's account has not yet been verified
    NotVerified,
    /// An error roccurred exchanging an OAuith refresh token for a new access token
    OAuthExchangeError,
    /// The requested OAuth provider is not supported
    OAuthProviderNotSupported,
    /// The OAuth session provided in a request was invalid or expired
    OAuthSessionExpired,
    /// The OAuth session provided in a request was not found
    OAuthSessionNotFound,
    /// The client requested a sort order that is not supported for the given model
    OrderBy,
    /// The password and confirmation fields supplied by the client do not match.
    PasswordConfirmMismatch,
    /// Internal error with the password hashing mechanism
    PasswordHasherError,
    /// Failed to start the server
    ServerStart,
    /// Internal error with the session backend
    SessionBackend,
    /// Error while shutting down the server
    Shutdown,
    /// Creation of new user accounts is disabled
    SignupDisabled,
    /// The user is not logged in
    Unauthenticated,
    /// Internal error while creating a user
    UserCreationError,
    /// The requested user does not exist
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
            Self::OAuthProviderNotSupported => "oauth_provider_not_supported",
            Self::OAuthSessionExpired => "oauth_session_expired",
            Self::OAuthSessionNotFound => "oauth_session_not_found",
            Self::OrderBy => "order_by",
            Self::PasswordConfirmMismatch => "password_mismatch",
            Self::PasswordHasherError => "password_hash_internal",
            Self::ServerStart => "server",
            Self::SessionBackend => "session_backend",
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
    /// Form data which should be returned even if the error is obfuscated
    pub form: Option<Arc<serde_json::Value>>,
}

impl ForceObfuscate {
    /// Create a new ForceObfuscate
    pub fn new(kind: impl Into<Cow<'static, str>>, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            form: None,
        }
    }

    /// Attach form data to the obfuscated error
    pub fn with_form(self, form: Arc<serde_json::Value>) -> Self {
        Self {
            form: Some(form),
            ..self
        }
    }

    /// A generic "Unauthenticated" error to return when the details of an authentication failure
    /// should be obfuscated.
    pub fn unauthenticated() -> Self {
        Self::new(ErrorKind::Unauthenticated, "Unauthenticated")
    }
}

/// Attach this to a [Report] to include form data when rendering the error response.
#[derive(Debug)]
pub struct FormDataResponse(pub Arc<serde_json::Value>);

impl FormDataResponse {
    /// Create a new FormDataResponse
    pub fn new(form: Arc<serde_json::Value>) -> Self {
        Self(form)
    }
}

impl<T> HttpError for error_stack::Report<T>
where
    T: HttpError + Send + Sync + 'static,
{
    type Detail = String;

    fn response_tuple(&self) -> (StatusCode, ErrorResponseData<Self::Detail>) {
        let err = ErrorResponseData::new(self.error_kind(), self.to_string(), self.error_detail());
        let err = if let Some(form_data) = self
            .frames()
            .find_map(|frame| frame.downcast_ref::<FormDataResponse>())
        {
            err.with_form(Some(form_data.0.clone()))
        } else {
            err
        };

        (self.status_code(), err)
    }

    fn obfuscate(&self) -> Option<ForceObfuscate> {
        self.current_context().obfuscate()
    }

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
    #[serde(skip_serializing_if = "Option::is_none")]
    form: Option<Arc<serde_json::Value>>,
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
            form: None,
            error: ErrorDetails {
                kind: kind.into(),
                message: message.into(),
                details: details.into(),
            },
        };

        event!(Level::ERROR, kind=%ret.error.kind, message=%ret.error.message, details=?ret.error.details);

        ret
    }

    /// Attach form details to the error response
    pub fn with_form(self, form: Option<Arc<serde_json::Value>>) -> Self {
        Self {
            form,
            error: self.error,
        }
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

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::auth::AuthError;

    #[test]
    fn report_form_data_attachment() {
        let err = Report::new(AuthError::Unauthenticated).attach(FormDataResponse::new(Arc::new(
            json!({ "email": "abc@example.com" }),
        )));

        let (_, data) = err.response_tuple();
        let form = data.form.unwrap();
        assert_eq!(form.as_ref(), &json!({ "email": "abc@example.com" }));
    }
}
