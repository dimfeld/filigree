use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use error_stack::Report;
use filigree::errors::HttpError;
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
}

impl From<Report<Error>> for Error {
    fn from(value: Report<Error>) -> Self {
        Error::WrapReport(value)
    }
}

impl HttpError for Error {
    fn error_kind(&self) -> &'static str {
        match self {
            Error::WrapReport(e) => e.current_context().error_kind(),
            Error::DbInit => "db_init",
            Error::Db => "db",
            Error::TaskQueue => "task_queue",
            Error::ServerStart => "server",
            Error::NotFound(_) => "not_found",
            Error::Shutdown => "shutdown",
            Error::ScheduledTask => "scheduled_task",
            Error::Filter => "invalid_filter",
            Error::AuthSubsystem => "auth",
            Error::MissingPermission(_) => "missing_permission",
        }
    }

    fn status_code(&self) -> StatusCode {
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
