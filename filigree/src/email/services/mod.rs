/// ReSend email service support
pub mod resend;

use async_trait::async_trait;
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
#[async_trait]
pub trait EmailService {
    /// Send an email
    async fn send(&self, email: Email) -> Result<(), EmailError>;
}

/// A service that manages email sending
pub struct EmailSender {
    default_from: String,
    service: Box<dyn EmailService>,
}

impl EmailSender {
    /// Create a new EmailSender
    pub fn new(default_from: String, service: Box<dyn EmailService>) -> Self {
        Self {
            default_from,
            service,
        }
    }

    /// Send an email, filling in any unset fields that have a default
    pub async fn send(&self, mut email: Email) -> Result<(), EmailError> {
        if email.from.is_empty() {
            email.from = self.default_from.clone();
        }

        self.service.send(email).await
    }
}
