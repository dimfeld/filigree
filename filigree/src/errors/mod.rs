mod http_error;
mod obfuscate_errors;
mod panic_handler;

pub use http_error::*;
pub use obfuscate_errors::*;
pub use panic_handler::*;
use thiserror::Error;

/// An error to return when failing to parse an order_by field.
/// This is used by the autogenerated code.
#[derive(Debug, Error)]
pub enum OrderByError {
    /// The field is not configured to allow sorting
    #[error("Field is not sortable")]
    InvalidField,
    /// This field restricts the direction in which it can be sorted.
    #[error("Field is not sortable in the requested direction")]
    InvalidDirection,
}

impl HttpError for OrderByError {
    type Detail = ();

    fn status_code(&self) -> hyper::StatusCode {
        hyper::StatusCode::BAD_REQUEST
    }

    fn error_kind(&self) -> &'static str {
        ErrorKind::OrderBy.as_str()
    }

    fn error_detail(&self) -> Self::Detail {
        ()
    }
}

/// Attempt to downcast_ref a [error_stack::Frame] into multiple error types,
/// and map the first matching type through a function. The map function should be generic on some
/// trait that the errors all implement, usually HttpError.
///
/// This would usually be used in a context like this:
///
/// ```
/// # use filigree::{
/// #    downref_report_frame,
/// #    errors::{HttpError, OrderByError},
/// #    uploads::UploadInspectorError,
/// # };
/// # #[derive(Debug)]
/// # struct Error {}
/// # impl std::error::Error for Error {}
/// # impl std::fmt::Display for Error {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         write!(f, "Error")
/// #     }
/// # }
/// fn error_code<E: HttpError>(err: &E) -> http::StatusCode {
///     err.status_code()
/// }
///
/// let report = error_stack::Report::new(OrderByError::InvalidField)
///     .change_context(Error{});
///
/// let status_code = report.frames().find_map(|frame| {
///     downref_report_frame!(
///         // The frame
///         frame,
///         // The function to call if we find a matching error.
///         error_code,
///         // The error types to try
///         OrderByError,
///         filigree::uploads::UploadInspectorError
///     )
/// });
///
/// // Should return the status code for OrderByError::InvalidField
/// assert_eq!(status_code, Some(http::StatusCode::BAD_REQUEST));
/// ```
#[macro_export]
macro_rules! downref_report_frame {
    ($frame:ident, $func:expr, $error_type:ty) => {
        $frame.downcast_ref::<$error_type>().map($func)
    };

    ($frame:ident, $func:expr, $error_type:ty, $($more_error_type:ty),+) => {
        $crate::downref_report_frame!($frame, $func, $error_type)
            $(
                .or_else(|| $crate::downref_report_frame!($frame, $func, $more_error_type))
            )+
    };
}
