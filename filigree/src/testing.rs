use async_trait::async_trait;
use reqwest::header::HeaderMap;
use tracing::{event, Level};

/// A password to use by default for users in unit tests.
pub const TEST_PASSWORD: &str = "the-password";
/// A hash created from [TEST_PASSWORD]
pub const TEST_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$7Pdxrl3fSrSIelBARWvE5g$0D8uG+7ezAU7CWbIZZ+IbrL3QrEXNZOAI4oYM5mWijk";

/// The ID of the admin user always created
#[cfg(feature = "string_user_ids")]
pub const ADMIN_USER_ID: &str = "e6ef5cb2-c361-4c21-9419-318fadac0fa4";

#[cfg(not(feature = "string_user_ids"))]
pub const ADMIN_USER_ID: crate::auth::UserId =
    crate::auth::UserId::from_uuid(uuid::Uuid::from_u128(0xE6EF5CB2C3614C219419318FADAC0FA4));

/// An HTTP client set up for ease of use in tests. It takes a base URL when constructed and
/// makes all requests relative to that base.
#[derive(Clone, Debug)]
pub struct TestClient {
    /// The base URL prepended to all requests
    pub base: String,
    /// The HTTP client actually used to make requests
    pub client: reqwest::Client,
}

impl TestClient {
    /// Create a new TestClient with a base URL
    pub fn new(base: impl Into<String>) -> TestClient {
        TestClient {
            base: base.into(),
            client: new_client_builder().build().expect("Building client"),
        }
    }

    /// Create a new TestClient from this one that uses the given client
    pub fn with_custom_client(&self, client: reqwest::Client) -> TestClient {
        TestClient {
            base: self.base.clone(),
            client,
        }
    }

    /// Create a new TestClient from this one that passes the given API key
    /// as a Bearer token.
    pub fn with_api_key(&self, api_key: &str) -> TestClient {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", api_key).parse().unwrap(),
        );

        TestClient {
            base: self.base.clone(),
            client: new_client_builder()
                .default_headers(headers)
                .build()
                .expect("Building client"),
        }
    }

    /// Create a new GET request
    pub fn get(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.get(format!("{}/{}", self.base, url.as_ref()))
    }

    /// Create a new POST request
    pub fn post(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.post(format!("{}/{}", self.base, url.as_ref()))
    }

    /// Create a new PUT request
    pub fn put(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.put(format!("{}/{}", self.base, url.as_ref()))
    }

    /// Create a new DELETE request
    pub fn delete(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client
            .delete(format!("{}/{}", self.base, url.as_ref()))
    }

    /// Create a new request with the given method
    pub fn request(
        &self,
        method: reqwest::Method,
        url: impl AsRef<str>,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}/{}", self.base, url.as_ref()))
    }
}

fn new_client_builder() -> reqwest::ClientBuilder {
    reqwest::ClientBuilder::new()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
}

/// Extensions for reqwest::Response
#[async_trait]
pub trait ResponseExt {
    /// If the response is an error, log it. This can be used in place of [error_for_status].
    async fn log_error(self) -> Result<reqwest::Response, reqwest::Error>;
}

#[async_trait]
impl ResponseExt for reqwest::Response {
    async fn log_error(self) -> Result<reqwest::Response, reqwest::Error> {
        if let Err(e) = self.error_for_status_ref() {
            let status = self.status().as_u16();
            let url = self.url().clone();
            let text = self.text().await.unwrap_or_default();
            event!(Level::ERROR, url=%url, %status, response=text);
            Err(e)
        } else {
            Ok(self)
        }
    }
}
