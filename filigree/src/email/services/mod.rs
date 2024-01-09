pub mod resend;

use futures::Future;
use thiserror::Error;

use super::Email;

/// Errors returned from an [EmailService]
#[derive(Debug, Error)]
pub enum EmailError {
    /// Email failed to send, without more detail
    #[error("Generic failure")]
    Failed,
    /// Email was too large to send
    #[error("Email was too large")]
    TooLarge,
}

/// A service that can send an email
pub trait EmailService {
    /// Send an email
    fn send(&self, email: Email) -> impl Future<Output = Result<(), EmailError>> + Send;
}
