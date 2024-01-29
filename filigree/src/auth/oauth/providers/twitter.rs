use async_trait::async_trait;
use error_stack::Report;
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use url::Url;

use super::{build_redirect_url, AuthorizeUrl, OAuthProvider, OAuthUserDetails};
use crate::auth::oauth::OAuthError;

/// OAuth provider for Twitter logins
pub struct TwitterOAuthProvider {
    client: BasicClient,
}

impl TwitterOAuthProvider {
    /// Set up the Twitter OAuth provider
    pub fn new(client_id: String, client_secret: String, redirect_base_url: &str) -> Self {
        let redirect_url = build_redirect_url(redirect_base_url, "twitter");

        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://twitter.com/i/oauth2/authorize".into()).unwrap(),
            Some(TokenUrl::new("https://api.twitter.com/2/oauth2/token".into()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap());

        Self { client }
    }
}

#[async_trait]
impl OAuthProvider for TwitterOAuthProvider {
    fn name(&self) -> &'static str {
        "twitter"
    }

    fn client(&self) -> &BasicClient {
        &self.client
    }

    fn authorize_url(&self) -> AuthorizeUrl {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("users.read".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        AuthorizeUrl {
            url,
            state,
            pkce_verifier: Some(pkce_verifier),
        }
    }

    async fn fetch_access_token(
        &self,
        authorization_code: String,
        pkce_verifier: String,
    ) -> Result<BasicTokenResponse, Report<OAuthError>> {
        super::fetch_access_token_with_pkce(&self.client, authorization_code, pkce_verifier).await
    }

    async fn fetch_user_details(
        &self,
        client: reqwest::Client,
        access_token: &str,
    ) -> Result<OAuthUserDetails, reqwest::Error> {
        #[derive(Deserialize)]
        struct TwitterUserDetails {
            id: String,
            name: Option<String>,
            username: Option<String>,
            profile_image_url: Option<Url>,
        }

        let user_details = client
            .get("https://api.twitter.com/2/users/me")
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json::<TwitterUserDetails>()
            .await?;

        Ok(OAuthUserDetails {
            login_id: user_details.id,
            name: user_details.name,
            avatar_url: user_details.profile_image_url,
            twitter_id: user_details.username,
            ..Default::default()
        })
    }
}
