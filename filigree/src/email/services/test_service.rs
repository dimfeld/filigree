use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tracing::{event, Level};

use super::{EmailError, EmailService};
use crate::email::Email;

/// An email service that doesn't send emails, but does save the generated emails for later
/// checking.
pub struct TestEmailService {
    /// The emails that have been sent
    pub emails: Arc<Mutex<Vec<Email>>>,
}

impl TestEmailService {
    /// Create a new TestEmailService
    pub fn new() -> Self {
        Self {
            emails: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait]
impl EmailService for TestEmailService {
    async fn send(&self, email: Email) -> Result<(), EmailError> {
        event!(Level::INFO, ?email, "Sending email");
        self.emails.lock().unwrap().push(email);
        Ok(())
    }
}
