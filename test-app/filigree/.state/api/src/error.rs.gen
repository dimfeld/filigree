use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use error_stack::Report;
use filigree::{
    auth::AuthError,
    errors::{ErrorKind as FilErrorKind, ForceObfuscate, HttpError},
};
use thiserror::Error;

/// The top-level error type from the platform
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to intialize database
    #[error("Failed to intialize database")]
    DbInit,
    /// Database error not otherwise handled
    #[error("Database error")]
    Db,
    /// Task queue error not otherwise handled
    #[error("Task Queue error")]
    TaskQueue,
    /// Failed to start the HTTP server
    #[error("Failed to start server")]
    ServerStart,
    /// Failure while shutting down
    #[error("Encountered error while shutting down")]
    Shutdown,
    /// Error running a scheduled task
    #[error("Error running scheduled task")]
    ScheduledTask,
    /// The requested item was not found
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("Invalid filter")]
    Filter,
    /// A wrapper around a Report<Error> to let it be returned from an Axum handler, since we can't
    /// implement IntoResponse on Report
    #[error("{0}")]
    WrapReport(Report<Error>),
    #[error("Missing Permission {0}")]
    MissingPermission(&'static str),
    #[error("Auth subsystem error")]
    AuthSubsystem,
    #[error("Login failure")]
    Login,
    /// An invalid Host header was passed
    #[error("Invalid host")]
    InvalidHostHeader,
}

impl From<Report<Error>> for Error {
    fn from(value: Report<Error>) -> Self {
        Error::WrapReport(value)
    }
}

impl Error {
    /// If this Error contains a Report<Error>, find an inner HttpError whose error data we may want to use.
    fn find_downstack_error(&self) -> Option<&AuthError> {
        let Error::WrapReport(report) = self else {
            return None;
        };

        // Currently this only applies to AuthError. Other errors don't need to pass through their
        // codes to the user.
        report.downcast_ref::<AuthError>()
    }
}

impl HttpError for Error {
    type Detail = String;

    fn error_kind(&self) -> &'static str {
        if let Some(e) = self.find_downstack_error() {
            return e.error_kind();
        }

        match self {
            Error::WrapReport(e) => e.current_context().error_kind(),
            Error::DbInit => FilErrorKind::DatabaseInit.as_str(),
            Error::Db => FilErrorKind::Database.as_str(),
            Error::TaskQueue => "task_queue",
            Error::ServerStart => FilErrorKind::ServerStart.as_str(),
            Error::NotFound(_) => FilErrorKind::NotFound.as_str(),
            Error::Shutdown => FilErrorKind::Shutdown.as_str(),
            Error::ScheduledTask => "scheduled_task",
            Error::Filter => "invalid_filter",
            Error::AuthSubsystem => "auth",
            Error::Login => FilErrorKind::Unauthenticated.as_str(),
            Error::MissingPermission(_) => FilErrorKind::Unauthenticated.as_str(),
            Error::InvalidHostHeader => FilErrorKind::InvalidHostHeader.as_str(),
        }
    }

    fn obfuscate(&self) -> Option<ForceObfuscate> {
        if let Some(e) = self.find_downstack_error() {
            return e.obfuscate();
        }

        match self {
            Error::InvalidHostHeader => Some(ForceObfuscate::new(
                FilErrorKind::BadRequest,
                "Invalid Request",
            )),
            _ => None,
        }
    }

    fn status_code(&self) -> StatusCode {
        if let Some(e) = self.find_downstack_error() {
            return e.status_code();
        }

        match self {
            Error::WrapReport(e) => e.current_context().status_code(),
            Error::DbInit => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Db => StatusCode::INTERNAL_SERVER_ERROR,
            Error::TaskQueue => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ServerStart => StatusCode::INTERNAL_SERVER_ERROR,
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::Shutdown => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ScheduledTask => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Filter => StatusCode::BAD_REQUEST,
            Error::AuthSubsystem => StatusCode::INTERNAL_SERVER_ERROR,
            Error::MissingPermission(_) => StatusCode::FORBIDDEN,
            Error::Login => StatusCode::UNAUTHORIZED,
            Error::InvalidHostHeader => StatusCode::BAD_REQUEST,
        }
    }

    fn error_detail(&self) -> String {
        match self {
            Error::WrapReport(e) => e.error_detail(),
            _ => String::new(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.to_response()
    }
}

pub enum ErrorKind {
    TaskQueue,
    ScheduledTask,
    Filter,
    AuthSubsystem,
    Login,
}

impl ErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::TaskQueue => "task_queue",
            ErrorKind::ScheduledTask => "scheduled_task",
            ErrorKind::Filter => "invalid_filter",
            ErrorKind::AuthSubsystem => "auth",
            ErrorKind::Login => "auth",
        }
    }
}
