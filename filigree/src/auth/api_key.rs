mod queries;

use base64::{display::Base64Display, engine::GeneralPurpose, Engine};
use chrono::{DateTime, Utc};
pub use queries::*;
use serde::Deserialize;
use sha3::Digest;
use uuid::Uuid;

use super::{AuthError, OrganizationId, UserId};

/// All the data stored for an API key, except the hash
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ApiKey {
    /// The ID of the key
    pub api_key_id: Uuid,
    /// The organization that this key belongs to
    pub organization_id: OrganizationId,
    /// The user that this key belongs to
    pub user_id: Option<UserId>,
    /// Whether this key should use the permissions of the user, or have its
    /// own set of permissions just for this key.
    pub inherits_user_permissions: bool,
    /// A description of the key
    pub description: String,
    /// Whether the key is enabled. Inactive keys can not be used.
    pub active: bool,
    /// When the key will expire
    pub expires_at: DateTime<Utc>,
}

/// A submission to update an API key
#[derive(Clone, Debug, Deserialize, sqlx::FromRow)]
pub struct ApiKeyUpdateBody {
    /// The description of the key
    pub description: Option<String>,
    /// Whether the key is active or not
    pub active: Option<bool>,
    /// When the key will expire
    pub expires_at: Option<DateTime<Utc>>,
}

/// A generated API key
/// A key consists of an ID and a random string.
/// The ID is used to look up the key in the database. The random portion is used to prevent tampering.
/// This entire key is hashed, and both the ID and the hash are compared against
/// in the database.
pub struct ApiKeyData {
    /// The ID of the key
    pub api_key_id: Uuid,
    /// The hash of the key.
    pub hash: Vec<u8>,
    /// The full representation of the key, which the user passes in to the API.
    /// This is not stored in the database.
    pub key: String,
}

const B64_ENGINE: GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

impl ApiKeyData {
    /// Create a new API key
    pub fn new() -> ApiKeyData {
        let id = Uuid::now_v7();
        let base64_id = Base64Display::new(id.as_bytes(), &B64_ENGINE);
        let random_id = Uuid::new_v4();
        let random = Base64Display::new(random_id.as_bytes(), &B64_ENGINE);
        let key = format!("{base64_id}.{random}");
        let hash = hash_key(&key);

        ApiKeyData {
            api_key_id: id,
            key,
            hash,
        }
    }
}

fn hash_key(key: &str) -> Vec<u8> {
    let mut hasher = sha3::Sha3_512::default();
    hasher.update(key.as_bytes());
    hasher.finalize().to_vec()
}

/// Parse an API key and into the constituent ID and hash.
pub fn decode_key(key: &str) -> Result<(Uuid, Vec<u8>), AuthError> {
    // Should be a pair of UUIDs base64 encoded and joined with '.'
    if key.len() != 45 {
        return Err(AuthError::ApiKeyFormat);
    }

    let hash = hash_key(key);
    let id_portion = key.split_once('.').ok_or(AuthError::InvalidApiKey)?.0;
    let api_key_bytes = B64_ENGINE
        .decode(id_portion.as_bytes())
        .map_err(|_| AuthError::InvalidApiKey)?;
    let api_key_id = Uuid::from_slice(&api_key_bytes).map_err(|_| AuthError::ApiKeyFormat)?;

    Ok((api_key_id, hash))
}

/// Lookup an API token given the bearer token form that the user provides.
pub async fn lookup_api_key_from_bearer_token(
    pool: &sqlx::PgPool,
    key: &str,
) -> Result<ApiKey, AuthError> {
    let (api_key_id, hash) = decode_key(key)?;
    queries::get_api_key(pool, &api_key_id, &hash).await
}

#[cfg(test)]
mod tests {

    use super::{decode_key, ApiKeyData};

    #[test]
    fn valid_key() {
        let data = ApiKeyData::new();

        let (api_key_id, hash) = decode_key(&data.key).expect("decoding key");
        assert_eq!(api_key_id, data.api_key_id, "api_key_id");
        assert_eq!(hash, data.hash, "hash");
    }

    #[test]
    fn bad_key() {
        let data = ApiKeyData::new();

        // Alter the key.
        let mut key = data.key;
        key.pop();
        key.push('a');

        let (api_key_id, hash) = decode_key(&key).expect("decoding key");
        assert_eq!(api_key_id, data.api_key_id, "api_key_id");
        assert_ne!(hash, data.hash, "hash");
    }

    #[test]
    fn bad_length() {
        let data = ApiKeyData::new();

        let mut key = String::from(&data.key);
        key.push('a');
        decode_key(&key).expect_err("length too high");

        key.pop();
        key.pop();
        decode_key(&key).expect_err("length too low");
    }
}
