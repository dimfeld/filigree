use reqwest::header::HeaderMap;
use uuid::Uuid;

use crate::auth::{OrganizationId, UserId};

pub const TEST_PASSWORD: &str = "the-password";
/// A hash created from [TEST_PASSWORD]
pub const TEST_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$7Pdxrl3fSrSIelBARWvE5g$0D8uG+7ezAU7CWbIZZ+IbrL3QrEXNZOAI4oYM5mWijk";

/// The ID of the admin user always created
pub const ADMIN_USER_ID: UserId =
    UserId::from_uuid(Uuid::from_u128(0xE6EF5CB2C3614C219419318FADAC0FA4));
pub const MAIN_ORG_ID: OrganizationId =
    OrganizationId::from_uuid(Uuid::from_u128(0x4397EC8413E64EABA6278D82CA400B76));
/// An alterrnate organization to store objects that the user should not see.
pub const OTHER_ORG_ID: OrganizationId =
    OrganizationId::from_uuid(Uuid::from_u128(0xDEDAF8557C09459F981097E2AD06F052));

#[derive(Clone, Debug)]
pub struct TestUser {
    pub user_id: UserId,
    pub organization_id: OrganizationId,
    pub password: String,
    pub api_key: String,
    pub client: TestClient,
}

/// An HTTP client set up for ease of use in tests
#[derive(Clone, Debug)]
pub struct TestClient {
    /// The base URL prepended to all requests
    pub base: String,
    /// The HTTP client actually used to make requests
    pub client: reqwest::Client,
}

impl TestClient {
    pub fn new(base: impl Into<String>) -> TestClient {
        TestClient {
            base: base.into(),
            client: reqwest::ClientBuilder::new()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Building client"),
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
            client: reqwest::ClientBuilder::new()
                .default_headers(headers)
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Building client"),
        }
    }

    pub fn get(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.get(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn post(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.post(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn put(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client.put(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn delete(&self, url: impl AsRef<str>) -> reqwest::RequestBuilder {
        self.client
            .delete(format!("{}/{}", self.base, url.as_ref()))
    }

    pub fn request(
        &self,
        method: reqwest::Method,
        url: impl AsRef<str>,
    ) -> reqwest::RequestBuilder {
        self.client
            .request(method, format!("{}/{}", self.base, url.as_ref()))
    }
}
