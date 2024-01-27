use std::borrow::Cow;

use async_trait::async_trait;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl,
};
use url::Url;

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

/// Configuration to authenticate with an OAuth provider
#[async_trait]
pub trait OAuthProvider {
    /// The name of the service
    fn name(&self) -> &'static str;

    /// A reference to the OAuth client
    fn client(&self) -> &BasicClient;

    /// Generate a URL to redirect the user to to log in.
    fn authorize_url(&self) -> (Url, CsrfToken);

    /// Get user info for an OAuth user
    async fn fetch_user_details(
        &self,
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

    fn authorize_url(&self) -> (Url, CsrfToken) {
        self.client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("user:read".to_string()))
            .url()
    }



    async fn fetch_user_details(
        &self,
        access_token: &str,
    ) -> Result<OAuthUserDetails, reqwest::Error> {
        todo!()
    }
}
