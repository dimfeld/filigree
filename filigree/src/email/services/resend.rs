use async_trait::async_trait;
use serde::Serialize;

use super::{EmailError, EmailService};
use crate::email::{Email, EmailAttachment};

/// Sends email using Resend (resend.com)
pub struct ResendEmailService {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl ResendEmailService {
    /// Create a new ResendEmailService at the normal base URL
    pub fn new(token: String) -> Self {
        Self {
            base_url: "https://api.resend.com".to_string(),
            client: reqwest::Client::new(),
            token,
        }
    }

    /// Create a new ResendEmailService, overriding the base URL
    pub fn new_with_base_url(base_url: String, token: String) -> Self {
        Self {
            base_url,
            token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EmailService for ResendEmailService {
    async fn send(&self, email: Email) -> Result<(), EmailError> {
        let body = ResendEmailBody {
            from: email.from,
            to: email.to,
            reply_to: email.reply_to,
            cc: email.cc,
            bcc: email.bcc,
            subject: email.subject,
            text: email.text,
            html: email.html,
            attachments: email.attachments,
            tags: email
                .tags
                .into_iter()
                .map(|tag| Tag { name: tag })
                .collect(),
        };

        self.client
            .post(format!("{}/emails", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|_| EmailError::Failed)?
            .error_for_status()
            // TODO better error decoding -- https://resend.com/docs/api-reference/errors
            .map_err(|_| EmailError::Failed)?;

        Ok(())
    }
}

#[derive(Serialize)]
struct ResendEmailBody {
    from: String,
    to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cc: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    bcc: Vec<String>,
    subject: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    html: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<EmailAttachment>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<Tag>,
}

#[derive(Serialize)]
struct Tag {
    name: String,
}
