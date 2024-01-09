use async_trait::async_trait;

use super::{EmailError, EmailService};
use crate::email::Email;

/// An email service that doesn't send emails. Can be useful when first starting out a project.
pub struct NoopEmailService {}

#[async_trait]
impl EmailService for NoopEmailService {
    async fn send(&self, _email: Email) -> Result<(), EmailError> {
        Ok(())
    }
}
