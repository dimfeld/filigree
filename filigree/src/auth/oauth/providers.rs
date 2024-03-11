use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use oauth2::{
    basic::{BasicClient, BasicTokenResponse},
    reqwest::async_http_client,
    AuthorizationCode, CsrfToken, PkceCodeVerifier,
};
use url::Url;

use super::OAuthError;
use crate::config::prefixed_env_var;

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
pub trait OAuthProvider: Send + Sync + 'static {
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

/// Create an OAuth2 redirect URL for a provider
pub fn build_redirect_url(base: &str, provider_name: &str) -> String {
    format!("{base}/{provider_name}/callback")
}

/// Create all the supported OAuth providers, inspecting the environment variables to determine
/// which ones are configured.
pub fn create_supported_providers(
    env_prefix: &str,
    redirect_base_url: &str,
) -> Vec<Box<dyn OAuthProvider>> {
    let github_provider = match (
        prefixed_env_var(env_prefix, "OAUTH_GITHUB_CLIENT_ID"),
        prefixed_env_var(env_prefix, "OAUTH_GITHUB_CLIENT_SECRET"),
    ) {
        (Ok(client_id), Ok(client_secret)) => Some(Box::new(GitHubOAuthProvider::new(
            client_id,
            client_secret,
            redirect_base_url,
        )) as Box<dyn OAuthProvider>),
        _ => None,
    };

    let google_provider = match (
        prefixed_env_var(env_prefix, "OAUTH_GOOGLE_CLIENT_ID"),
        prefixed_env_var(env_prefix, "OAUTH_GOOGLE_CLIENT_SECRET"),
    ) {
        (Ok(client_id), Ok(client_secret)) => Some(Box::new(GoogleOAuthProvider::new(
            client_id,
            client_secret,
            redirect_base_url,
        )) as Box<dyn OAuthProvider>),
        _ => None,
    };

    let twitter_provider = match (
        prefixed_env_var(env_prefix, "OAUTH_TWITTER_CLIENT_ID"),
        prefixed_env_var(env_prefix, "OAUTH_TWITTER_CLIENT_SECRET"),
    ) {
        (Ok(client_id), Ok(client_secret)) => Some(Box::new(TwitterOAuthProvider::new(
            client_id,
            client_secret,
            redirect_base_url,
        )) as Box<dyn OAuthProvider>),
        _ => None,
    };

    [github_provider, google_provider, twitter_provider]
        .into_iter()
        .flatten()
        .collect()
}
