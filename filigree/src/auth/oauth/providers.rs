use std::{borrow::Cow, sync::Arc};

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use url::Url;

use super::OAuthError;

/// User information from an OAuth provider
pub struct OAuthUserDetails {
    /// An ID for the user
    pub login_id: String,
    /// The user's name
    pub name: Option<String>,
    /// The user's email
    pub email: Option<String>,
    /// An avatar image for this user
    pub avatar_url: Option<Url>,
}

pub struct AuthorizeUrl {
    pub url: Url,
    pub state: CsrfToken,
    pub pkce_verifier: Option<PkceCodeVerifier>,
}

/// Configuration to authenticate with an OAuth provider
#[async_trait]
pub trait OAuthProvider {
    /// The name of the service
    fn name(&self) -> &'static str;

    /// A reference to the OAuth client
    fn client(&self) -> &BasicClient;

    /// Generate a URL to redirect the user to to log in.
    fn authorize_url(&self) -> AuthorizeUrl;

    async fn fetch_access_token(
        &self,
        authorization_code: String,
        pkce_verifier: String,
    ) -> Result<BasicTokenResponse, Report<OAuthError>>;

    /// Get user info for an OAuth user
    async fn fetch_user_details(
        &self,
        client: reqwest::Client,
        access_token: &str,
    ) -> Result<OAuthUserDetails, reqwest::Error>;
}

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
        self.client()
            .exchange_code(AuthorizationCode::new(authorization_code))
            .request_async(async_http_client)
            .await
            .change_context(OAuthError::ExchangeError)
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
        })
    }
}

struct TwitterOAuthProvider {
    client: BasicClient,
}

impl TwitterOAuthProvider {
    /// Set up the Twitter OAuth provider
    pub fn new(client_id: String, client_secret: String, redirect_base_url: String) -> Self {
        let mut redirect_url = Url::parse(&redirect_base_url).unwrap();
        redirect_url.set_path("/api/auth/oauth/twitter/callback");

        let client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://twitter.com/i/oauth2/authorize".into()).unwrap(),
            Some(TokenUrl::new("https://api.twitter.com/2/oauth2/token".into()).unwrap()),
        );

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
        let verifier = PkceCodeVerifier::new(pkce_verifier);
        self.client()
            .exchange_code(AuthorizationCode::new(authorization_code))
            .set_pkce_verifier(verifier)
            .request_async(async_http_client)
            .await
            .change_context(OAuthError::ExchangeError)
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

        todo!();
    }
}
