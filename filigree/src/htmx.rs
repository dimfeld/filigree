//! [htmx](https://htmx.org) Utilities

use http::StatusCode;

/// A StatusCode of 286, which indicates that htmx should stop polling
pub fn status_stop_polling() -> http::StatusCode {
    StatusCode::from_u16(286).unwrap()
}
