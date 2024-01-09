use serde::Serialize;

/// Email sending services
pub mod services;

/// An email to be sent
#[derive(Debug, Default)]
pub struct Email {
    /// Sender of the email
    pub from: String,
    /// Recipients of the email
    pub to: Vec<String>,
    /// For emails where the reply address differs from the From field.
    pub reply_to: Option<String>,
    /// CC email addresses
    pub cc: Vec<String>,
    /// BCC email addresses
    pub bcc: Vec<String>,
    /// Subject of the email
    pub subject: String,
    /// Plain text content of the email
    pub text: String,
    /// HTML content of the email
    pub html: String,
    /// Attachments for this email
    pub attachments: Vec<EmailAttachment>,
    /// Tags for this email, for those services that support them.
    pub tags: Vec<String>,
}

/// An attachment to an email
#[derive(Serialize)]
pub struct EmailAttachment {
    filename: String,
    content: Vec<u8>,
}

impl std::fmt::Debug for EmailAttachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailAttachment")
            .field("filename", &self.filename)
            .field("content_length", &self.content.len())
            .finish()
    }
}
