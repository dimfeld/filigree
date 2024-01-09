use serde::Serialize;
use typed_builder::TypedBuilder;

/// Email sending services
pub mod services;

/// An email to be sent
#[derive(Debug, Default, TypedBuilder)]
#[builder(doc, field_defaults(default))]
pub struct Email {
    /// Sender of the email
    #[builder(setter(into))]
    pub from: String,
    /// Recipients of the email
    #[builder(!default, setter(into, suffix="_vec"),
        via_mutators(init= Vec::new()),
        mutators(
            fn to(&mut self, to: impl ToString) {
                self.to.push(to.to_string())
            }

            fn to_vec(&mut self, to: Vec<String>) {
                self.to = to
            }
    ))]
    pub to: Vec<String>,
    /// For emails where the reply address differs from the From field.
    #[builder(setter(strip_option, into))]
    pub reply_to: Option<String>,
    /// CC email addresses
    #[builder(setter(into, suffix="_vec"),
        via_mutators(init = Vec::new()),
        mutators(
        fn cc(self, cc: impl ToString) {
            self.cc.push(cc.to_string())
        }

        pub fn cc_vec(&mut self, cc: Vec<String>) {
            self.cc = cc
        }
    ))]
    pub cc: Vec<String>,
    /// BCC email addresses
    #[builder(setter(into, suffix="_vec"),
        via_mutators(init = Vec::new()),
        mutators(
            pub fn bcc(&mut self, bcc: impl ToString) {
                self.bcc.push(bcc.to_string())
            }

            pub fn bcc_vec(&mut self, bcc: Vec<String>) {
                self.bcc = bcc
            }
    ))]
    pub bcc: Vec<String>,
    /// Subject of the email
    #[builder(!default, setter(into))]
    pub subject: String,
    /// Plain text content of the email
    #[builder(setter(into))]
    pub text: String,
    /// HTML content of the email
    #[builder(setter(into))]
    pub html: String,
    /// Attachments for this email
    #[builder(
        via_mutators(init = Vec::new()),
        mutators(
            fn attachment(&mut self, attachment: EmailAttachment) {
                self.attachments.push(attachment)
            }

            fn attachments(&mut self, attachments: Vec<EmailAttachment>) {
                self.attachments = attachments
            }
    ))]
    pub attachments: Vec<EmailAttachment>,
    /// Tags for this email, for those services that support them.
    #[builder(
        via_mutators(init = Vec::new()),
        mutators(
            fn tag(&mut self, tag: impl ToString) {
                self.tags.push(tag.to_string())
            }
    ))]
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

#[cfg(test)]
mod test {
    #[test]
    fn email_builder() {
        let email = super::Email::builder()
            .from("abc@def.com")
            .to("someone@example.com")
            .to("someone@else.com")
            .cc("manager@example.com")
            .bcc_vec(vec!["crm@example.com".to_string()])
            .html("<b>Hello</b>!")
            .subject("Hello!")
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
