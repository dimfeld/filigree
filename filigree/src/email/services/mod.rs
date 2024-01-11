/// An email service that does nothing, for testing and early development
pub mod noop_service;
/// ReSend email service support
#[cfg(feature = "email_resend")]
pub mod resend;
/// An email sender for tests, which doesn't send the emails but does save the content for later
/// inspection.
pub mod test_service;

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use thiserror::Error;

use super::{templates::EmailTemplate, Email};

/// Errors returned from an [EmailService]
#[derive(Debug, Error)]
pub enum EmailError {
    /// Error while rendering an email template
    #[error("Template render error")]
    Rendering,
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
    templates: tera::Tera,
    service: Box<dyn EmailService>,
}

impl EmailSender {
    /// Create a new EmailSender
    pub fn new(
        default_from: String,
        templates: tera::Tera,
        service: Box<dyn EmailService>,
    ) -> Self {
        Self {
            default_from,
            templates,
            service,
        }
    }

    /// Send an email, filling in any unset fields that have a default
    pub async fn send(&self, mut email: Email) -> Result<(), Report<EmailError>> {
        if email.from.is_empty() {
            email.from = self.default_from.clone();
        }

        if !email.html.is_empty() {
            let inliner = css_inline::CSSInliner::options()
                .load_remote_stylesheets(false)
                .build();
            email.html = inliner
                .inline(&email.html)
                .change_context(EmailError::Rendering)?;
        }

        self.service.send(email).await?;
        Ok(())
    }

    /// Render an email template and send the email.
    pub async fn send_template(
        &self,
        to: String,
        template: impl EmailTemplate,
    ) -> Result<(), Report<EmailError>> {
        let email = template
            .into_email(&self.templates, to)?
            .from(self.default_from.clone())
            .build();
        self.send(email).await?;
        Ok(())
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
