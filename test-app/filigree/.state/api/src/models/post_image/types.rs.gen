#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use sqlx_transparent_json_decode::sqlx_json_decode;

use super::PostImageId;
use crate::models::{organization::OrganizationId, post::PostId};

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct PostImage {
    pub id: PostImageId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub file_storage_key: String,
    pub file_storage_bucket: String,
    pub file_original_name: Option<String>,
    pub file_size: Option<i64>,
    pub file_hash: Option<Vec<u8>>,
    pub post_id: PostId,
    pub _permission: ObjectPermission,
}

pub type PostImagePopulatedGet = PostImage;

pub type PostImagePopulatedList = PostImage;

pub type PostImageCreateResult = PostImage;

impl PostImage {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> PostImageId {
        <PostImageId as Default>::default().into()
    }

    pub fn default_organization_id() -> crate::models::organization::OrganizationId {
        <crate::models::organization::OrganizationId as Default>::default().into()
    }

    pub fn default_updated_at() -> chrono::DateTime<chrono::Utc> {
        <chrono::DateTime<chrono::Utc> as Default>::default().into()
    }

    pub fn default_created_at() -> chrono::DateTime<chrono::Utc> {
        <chrono::DateTime<chrono::Utc> as Default>::default().into()
    }

    pub fn default_file_storage_key() -> String {
        <String as Default>::default().into()
    }

    pub fn default_file_storage_bucket() -> String {
        <String as Default>::default().into()
    }

    pub fn default_file_original_name() -> Option<String> {
        None
    }

    pub fn default_file_size() -> Option<i64> {
        None
    }

    pub fn default_file_hash() -> Option<Vec<u8>> {
        None
    }

    pub fn default_post_id() -> PostId {
        <PostId as Default>::default().into()
    }
}

sqlx_json_decode!(PostImage);

impl Default for PostImage {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            file_storage_key: Self::default_file_storage_key(),
            file_storage_bucket: Self::default_file_storage_bucket(),
            file_original_name: Self::default_file_original_name(),
            file_size: Self::default_file_size(),
            file_hash: Self::default_file_hash(),
            post_id: Self::default_post_id(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for PostImage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("PostImage", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("file_original_name", &self.file_original_name)?;
        state.serialize_field("file_size", &self.file_size)?;
        state.serialize_field("file_hash", &self.file_hash)?;
        state.serialize_field("post_id", &self.post_id)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]
#[cfg_attr(test, derive(Serialize))]
pub struct PostImageCreatePayloadAndUpdatePayload {
    pub id: Option<PostImageId>,
    pub post_id: PostId,
}

pub type PostImageCreatePayload = PostImageCreatePayloadAndUpdatePayload;

pub type PostImageUpdatePayload = PostImageCreatePayloadAndUpdatePayload;

impl PostImageCreatePayloadAndUpdatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> Option<PostImageId> {
        None
    }

    pub fn default_post_id() -> PostId {
        <PostId as Default>::default().into()
    }
}

impl Default for PostImageCreatePayloadAndUpdatePayload {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            post_id: Self::default_post_id(),
        }
    }
}
