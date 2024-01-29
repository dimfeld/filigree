use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    reqwest::async_http_client,
    AuthorizationCode, CsrfToken, PkceCodeVerifier,
};
use url::Url;

use super::OAuthError;

mod github;
mod google;
mod twitter;

pub use github::*;
pub use google::*;
pub use twitter::*;

/// User information from an OAuth provider
#[derive(Default, Debug)]
pub struct OAuthUserDetails {
    /// An ID for the user
    pub login_id: String,
    /// The user's name
    pub name: Option<String>,
    /// The user's email
    pub email: Option<String>,
    /// An avatar image for this user
    pub avatar_url: Option<Url>,
    /// The user's Twitter ID
    pub twitter_id: Option<String>,
}

/// An authorization URL and state generated in the URL to save for handling the response.
pub struct AuthorizeUrl {
    /// The full URL to send the user's browser to
    pub url: Url,
    /// A random string to use as the state. This is the key in the oauth_authorization_sessions table
    pub state: CsrfToken,
    /// The PKCE verifier code, if applicable.
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

    /// Exchange the authorization token for an access token.
    /// If PKCE is in use, `pkce_verifier` will be the appropriate value for this request.
    /// If not, `pkce_verifier` will be an empty string.
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

/// Helper function to exchange an authorization token for an access token, without PKCE.
pub async fn fetch_access_token_simple(
    client: &BasicClient,
    authorization_code: String,
) -> Result<BasicTokenResponse, Report<OAuthError>> {
    client
        .exchange_code(AuthorizationCode::new(authorization_code))
        .request_async(async_http_client)
        .await
        .change_context(OAuthError::ExchangeError)
}

/// Helper function to exchange an authorization token for an access token, with PKCE.
pub async fn fetch_access_token_with_pkce(
    client: &BasicClient,
    authorization_code: String,
    pkce_verifier: String,
) -> Result<BasicTokenResponse, Report<OAuthError>> {
    let verifier = PkceCodeVerifier::new(pkce_verifier);
    client
        .exchange_code(AuthorizationCode::new(authorization_code))
        .set_pkce_verifier(verifier)
        .request_async(async_http_client)
        .await
        .change_context(OAuthError::ExchangeError)
}

/// Create an OAuth 2 redirect URL for a provider
pub fn build_redirect_url(base: &str, provider_name: &str) -> String {
    format!("{base}/api/auth/oauth/{provider_name}/callback")
}
