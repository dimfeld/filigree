use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use url::Url;

use super::{AuthorizeUrl, OAuthProvider, OAuthUserDetails};
use crate::auth::oauth::OAuthError;

struct GitHubOAuthProvider {
    client: BasicClient,
}

impl GitHubOAuthProvider {
    /// Set up the Github OAuth provider
    pub fn new(client_id: String, client_secret: String, redirect_base_url: String) -> Self {
        let mut redirect_url = Url::parse(&redirect_base_url).unwrap();
        redirect_url.set_path("/api/auth/oauth/github/callback");

        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://github.com/login/oauth/authorize".into()).unwrap(),
            Some(TokenUrl::new("https://github.com/login/oauth/access_token".into()).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::from_url(redirect_url));

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
            id: String,
            email: Option<String>,
            name: Option<String>,
            avatar_url: Option<Url>,
        }

        let user_details = client
            .get("https://github.com/api/user")
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json::<GithubUserDetails>()
            .await?;

        Ok(OAuthUserDetails {
            login_id: user_details.id,
            name: user_details.name,
            email: user_details.email,
            avatar_url: user_details.avatar_url,
            ..Default::default()
        })
    }
}
