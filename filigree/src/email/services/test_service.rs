use std::sync::Mutex;

use async_trait::async_trait;

use super::{EmailError, EmailService};
use crate::email::Email;

/// An email service that doesn't send emails, but does save the generated emails for later
/// checking.
pub struct TestEmailService {
    /// The emails that have been sent
    pub emails: Mutex<Vec<Email>>,
}

impl TestEmailService {
    /// Create a new TestEmailService
    pub fn new() -> Self {
        Self {
            emails: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl EmailService for TestEmailService {
    async fn send(&self, email: Email) -> Result<(), EmailError> {
        println!("Sending email: {:?}", email);
        self.emails.lock().unwrap().push(email);
        Ok(())
    }
}
