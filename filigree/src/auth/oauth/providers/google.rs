use async_trait::async_trait;
use error_stack::Report;
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use url::Url;

use super::{build_redirect_url, AuthorizeUrl, OAuthProvider, OAuthUserDetails};
use crate::auth::oauth::OAuthError;

/// OAuth provider for Google logins
pub struct GoogleOAuthProvider {
    client: BasicClient,
}

impl GoogleOAuthProvider {
    /// Set up the Github OAuth provider
    pub fn new(client_id: String, client_secret: String, redirect_base_url: &str) -> Self {
        let redirect_url = build_redirect_url(redirect_base_url, "google");

        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into()).unwrap(),
            Some(TokenUrl::new("https://oauth2.googleapis.com/token".into()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap());

        Self { client }
    }
}

#[async_trait]
impl OAuthProvider for GoogleOAuthProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    fn client(&self) -> &BasicClient {
        &self.client
    }

    fn authorize_url(&self) -> AuthorizeUrl {
        let (url, state) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/userinfo.profile".to_string(),
            ))
            .url();

        AuthorizeUrl {
            url,
            state,
            pkce_verifier: None,
        }
    }

    async fn fetch_access_token(
        &self,
        authorization_code: String,
        _pkce_verifier: String,
    ) -> Result<BasicTokenResponse, Report<OAuthError>> {
        super::fetch_access_token_simple(&self.client, authorization_code).await
    }

    async fn fetch_user_details(
        &self,
        client: reqwest::Client,
        access_token: &str,
    ) -> Result<OAuthUserDetails, reqwest::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FieldMetadata {
            primary: bool,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Name {
            metadata: FieldMetadata,
            display_name: String,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct EmailAddress {
            metadata: FieldMetadata,
            value: String,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Photo {
            metadata: FieldMetadata,
            url: String,
            default: bool,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Person {
            resource_name: String,
            email_addresses: Vec<EmailAddress>,
            names: Vec<Name>,
            photos: Vec<Photo>,
        }

        let user_details = client
            .get("https://people.googleapis.com/v1/people/me")
            .query(&[("personFields", "names,emailAddresses,photos")])
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json::<Person>()
            .await?;

        let name = user_details
            .names
            .iter()
            .find(|name| name.metadata.primary)
            .or(user_details.names.get(0))
            .map(|name| name.display_name.to_string());

        let email = user_details
            .email_addresses
            .iter()
            .find(|email| email.metadata.primary)
            .or(user_details.email_addresses.get(0))
            .map(|email| email.value.to_string());

        let avatar_url = user_details
            .photos
            .iter()
            .find(|photo| photo.metadata.primary)
            .or(user_details.photos.get(0))
            .filter(|photo| !photo.default)
            .and_then(|photo| Url::parse(&photo.url).ok());

        Ok(OAuthUserDetails {
            login_id: user_details.resource_name,
            name,
            email,
            avatar_url,
            ..Default::default()
        })
    }
}
