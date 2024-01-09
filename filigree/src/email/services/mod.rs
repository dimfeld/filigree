/// An email service that does nothing, for testing and early development
pub mod noop_service;
/// ReSend email service support
#[cfg(feature = "email_resend")]
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
pub trait EmailService: Send + Sync {
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

/// Create an [EmailService] given the name of the service and an API key
#[cfg(feature = "email_provider")]
pub fn email_service_from_name(name: &str, api_key: String) -> Box<dyn EmailService> {
    match name {
        "none" => Box::new(noop_service::NoopEmailService {}),
        #[cfg(feature = "email_resend")]
        "resend" => Box::new(resend::ResendEmailService::new(api_key)),
        _ => panic!("Unknown email service: {}", name),
    }
}
