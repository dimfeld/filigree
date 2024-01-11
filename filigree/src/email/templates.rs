use std::borrow::Cow;

use error_stack::{Report, ResultExt};
use serde::Serialize;

use super::{services::EmailError, EmailBuilder};

/// The HTML and Text content of an email
pub struct EmailContent {
    /// HTML content of the email
    pub html: String,
    /// Text content of the email
    pub text: String,
}

/// An Email Template
pub trait EmailTemplate {
    /// Generate a subject for the email
    fn subject(&self) -> String;

    /// Render plaintext and HTML for an email
    fn render(&self, renderer: &tera::Tera) -> Result<EmailContent, tera::Error>;

    /// Tags for this email
    fn tags(&self) -> Vec<String> {
        vec![]
    }

    /// Render an email from this template and set the to, subject, and tags fields.
    fn into_email(
        &self,
        renderer: &tera::Tera,
        to: String,
    ) -> Result<EmailBuilder, Report<EmailError>> {
        let EmailContent { html, text } = self
            .render(renderer)
            .change_context(EmailError::Rendering)?;
        let builder = super::EmailBuilder::new(to, self.subject())
            .html(html)
            .text(text)
            .tags(self.tags());
        Ok(builder)
    }
}

/// A helper function for [EmailTemplate] implementors to render a text and html template
pub fn render_template_pair(
    tera: &tera::Tera,
    data: &impl Serialize,
    html_path: &str,
    text_path: &str,
) -> Result<EmailContent, tera::Error> {
    let context = tera::Context::from_serialize(data)?;
    let html = tera.render(html_path, &context)?;
    let text = tera.render(text_path, &context)?;

    Ok(EmailContent { html, text })
}

/// Create a Tera instance from a set of templates, inlining CSS stylesheets on HTML pages.
pub fn create_templates(
    templates: impl Iterator<Item = (Cow<'static, str>, rust_embed::EmbeddedFile)>,
    css: Cow<'static, [u8]>,
) -> tera::Tera {
    let css = std::str::from_utf8(css.as_ref()).unwrap();
    let inliner = css_inline::CSSInliner::options()
        .load_remote_stylesheets(false)
        .extra_css(Some(css.into()))
        .build();

    let templates = templates
        .filter(|(name, _)| name.ends_with(".html") || name.ends_with(".txt"))
        .map(|(name, data)| {
            let data = match data.data {
                Cow::Borrowed(b) => Cow::Borrowed(std::str::from_utf8(b).unwrap()),
                Cow::Owned(s) => Cow::Owned(String::from_utf8(s).unwrap()),
            };

            let data = if name.ends_with(".html") {
                Cow::from(inliner.inline(&data).unwrap())
            } else {
                data
            };

            (name, data)
        })
        .collect::<Vec<_>>();

    let mut tera = tera::Tera::default();
    tera.add_raw_templates(templates).unwrap();

    tera
}
