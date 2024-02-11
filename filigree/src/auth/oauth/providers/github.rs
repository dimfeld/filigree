use async_trait::async_trait;
use error_stack::Report;
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use url::Url;

use super::{build_redirect_url, AuthorizeUrl, OAuthProvider, OAuthUserDetails};
use crate::{auth::oauth::OAuthError, inspect_response::InspectResponseError};

/// OAuth provider for Github logins
pub struct GitHubOAuthProvider {
    client: BasicClient,
}

impl GitHubOAuthProvider {
    /// Set up the Github OAuth provider
    pub fn new(client_id: String, client_secret: String, redirect_base_url: &str) -> Self {
        let redirect_url = build_redirect_url(redirect_base_url, "github");

        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://github.com/login/oauth/authorize".into()).unwrap(),
            Some(TokenUrl::new("https://github.com/login/oauth/access_token".into()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap());

        Self { client }
    }
}

#[async_trait]
impl OAuthProvider for GitHubOAuthProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn client(&self) -> &BasicClient {
        &self.client
    }

    fn authorize_url(&self) -> AuthorizeUrl {
        let (url, state) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("user:read".to_string()))
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
        struct GithubUserDetails {
            id: i64,
            email: Option<String>,
            name: Option<String>,
            avatar_url: Option<Url>,
        }

        let user_details = client
            .get("https://api.github.com/user")
            .bearer_auth(access_token)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .print_error_for_status()
            .await?
            .json::<GithubUserDetails>()
            .await?;

        Ok(OAuthUserDetails {
            login_id: user_details.id.to_string(),
            name: user_details.name,
            email: user_details.email,
            avatar_url: user_details.avatar_url,
            ..Default::default()
        })
    }
}
