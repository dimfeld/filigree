use error_stack::Report;
use serde::Serialize;

use self::services::{EmailError, EmailSender};

/// Email sending services
pub mod services;
/// Email template helpers
pub mod templates;

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
    /// Content of the email
    pub text: String,
    /// HTML content of the email
    pub html: String,
    /// Attachments for this email
    pub attachments: Vec<EmailAttachment>,
    /// Tags for this email, for those services that support them.
    pub tags: Vec<String>,
}

/// An attachment to an email
#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Serialize)]
pub struct EmailAttachment {
    /// The name of the attachment
    pub filename: String,
    /// The file itself
    pub content: Vec<u8>,
}

impl std::fmt::Debug for EmailAttachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailAttachment")
            .field("filename", &self.filename)
            .field("content_length", &self.content.len())
            .finish()
    }
}

/// A builder for an [Email]
pub struct EmailBuilder {
    email: Email,
}

impl EmailBuilder {
    /// Create a new EmailBuilder with a to address and a subject
    pub fn new(to: impl ToString, subject: impl ToString) -> Self {
        Self {
            email: Email {
                to: vec![to.to_string()],
                subject: subject.to_string(),
                ..Default::default()
            },
        }
    }

    /// Create a new EmailBuilder with multiple to addresses and a subject
    pub fn new_to_multiple(to: Vec<String>, subject: impl ToString) -> Self {
        Self {
            email: Email {
                to,
                subject: subject.to_string(),
                ..Default::default()
            },
        }
    }

    /// Set the From address
    pub fn from(mut self, from: impl ToString) -> Self {
        self.email.from = from.to_string();
        self
    }

    /// Add an additional to address
    pub fn to(mut self, to: impl ToString) -> Self {
        self.email.to.push(to.to_string());
        self
    }

    /// Set the to addresses
    pub fn to_vec(mut self, to: Vec<String>) -> Self {
        self.email.to = to;
        self
    }

    /// Set the Reply-To email address
    pub fn reply_to(mut self, reply_to: impl ToString) -> Self {
        self.email.reply_to = Some(reply_to.to_string());
        self
    }

    /// Add a CC address
    pub fn cc(mut self, cc: impl ToString) -> Self {
        self.email.cc.push(cc.to_string());
        self
    }

    /// Set the CC addresses
    pub fn cc_vec(mut self, cc: Vec<String>) -> Self {
        self.email.cc = cc;
        self
    }

    /// Add a BCC address
    pub fn bcc(mut self, bcc: impl ToString) -> Self {
        self.email.bcc.push(bcc.to_string());
        self
    }

    /// Set the BCC addresses
    pub fn bcc_vec(mut self, bcc: Vec<String>) -> Self {
        self.email.bcc = bcc;
        self
    }

    /// Set the email subject
    pub fn subject(mut self, subject: impl ToString) -> Self {
        self.email.subject = subject.to_string();
        self
    }

    /// Set the HTML content
    pub fn html(mut self, html: impl ToString) -> Self {
        self.email.html = html.to_string();
        self
    }

    /// Set the plain text content
    pub fn text(mut self, text: impl ToString) -> Self {
        self.email.text = text.to_string();
        self
    }

    /// Add an attachment to the email
    pub fn attachment(mut self, attachment: EmailAttachment) -> Self {
        self.email.attachments.push(attachment);
        self
    }

    /// Set the attachments to the email
    pub fn attachments(mut self, attachments: Vec<EmailAttachment>) -> Self {
        self.email.attachments = attachments;
        self
    }

    /// Add a tag to the email
    pub fn tag(mut self, tag: impl ToString) -> Self {
        self.email.tags.push(tag.to_string());
        self
    }

    /// Set the tags on the email
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.email.tags = tags;
        self
    }

    /// Send the email to the configured email service
    pub async fn send(self, sender: &EmailSender) -> Result<(), Report<EmailError>> {
        sender.send(self.email).await
    }

    /// Return the email
    pub fn build(self) -> Email {
        self.email
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn email_builder() {
        let email = super::EmailBuilder::new("someone@example.com", "Hello!")
            .from("abc@def.com")
            .to("someone@else.com")
            .cc("manager@example.com")
            .bcc_vec(vec!["crm@example.com".to_string()])
            .html("<b>Hello</b>!")
            .attachment(super::EmailAttachment {
                filename: "hello.txt".to_string(),
                content: vec![1, 2, 3],
            })
            .build();

        assert_eq!(email.to, vec!["someone@example.com", "someone@else.com"]);
        assert_eq!(email.cc, vec!["manager@example.com"]);
        assert_eq!(email.bcc, vec!["crm@example.com"]);
        assert_eq!(email.subject, "Hello!");
        assert_eq!(email.html, "<b>Hello</b>!");
        assert_eq!(email.text, "");
        assert_eq!(
            email.attachments,
            vec![super::EmailAttachment {
                filename: "hello.txt".to_string(),
                content: vec![1, 2, 3],
            }]
        )
    }
}
