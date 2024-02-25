#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use sqlx_transparent_json_decode::sqlx_json_decode;

use super::PostId;
use crate::models::{
    comment::{Comment, CommentCreatePayload, CommentId, CommentUpdatePayload},
    organization::OrganizationId,
    poll::{Poll, PollCreatePayload, PollId, PollUpdatePayload},
    reaction::{Reaction, ReactionCreatePayload, ReactionId, ReactionUpdatePayload},
};

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct Post {
    pub id: PostId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub subject: String,
    pub body: String,
    pub _permission: ObjectPermission,
}

impl Post {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> PostId {
        <PostId as Default>::default().into()
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

    pub fn default_subject() -> String {
        <String as Default>::default().into()
    }

    pub fn default_body() -> String {
        <String as Default>::default().into()
    }
}

sqlx_json_decode!(Post);

impl Default for Post {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            subject: Self::default_subject(),
            body: Self::default_body(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for Post {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Post", 7)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("subject", &self.subject)?;
        state.serialize_field("body", &self.body)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]
#[cfg_attr(test, derive(Serialize))]
pub struct PostCreatePayloadAndUpdatePayload {
    pub id: Option<PostId>,
    pub subject: String,
    pub body: String,
}

pub type PostCreatePayload = PostCreatePayloadAndUpdatePayload;

pub type PostUpdatePayload = PostCreatePayloadAndUpdatePayload;

impl PostCreatePayloadAndUpdatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> Option<PostId> {
        None
    }

    pub fn default_subject() -> String {
        <String as Default>::default().into()
    }

    pub fn default_body() -> String {
        <String as Default>::default().into()
    }
}

impl Default for PostCreatePayloadAndUpdatePayload {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            subject: Self::default_subject(),
            body: Self::default_body(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct PostPopulatedGet {
    pub id: PostId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub subject: String,
    pub body: String,
    pub comment_ids: Vec<CommentId>,
    pub reactions: Vec<Reaction>,
    pub poll: Option<Poll>,
    pub _permission: ObjectPermission,
}

impl PostPopulatedGet {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> PostId {
        <PostId as Default>::default().into()
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

    pub fn default_subject() -> String {
        <String as Default>::default().into()
    }

    pub fn default_body() -> String {
        <String as Default>::default().into()
    }

    pub fn default_comment_ids() -> Vec<CommentId> {
        <Vec<CommentId> as Default>::default().into()
    }

    pub fn default_reactions() -> Vec<Reaction> {
        <Vec<Reaction> as Default>::default().into()
    }

    pub fn default_poll() -> Option<Poll> {
        None
    }
}

sqlx_json_decode!(PostPopulatedGet);

impl Default for PostPopulatedGet {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            subject: Self::default_subject(),
            body: Self::default_body(),
            comment_ids: Self::default_comment_ids(),
            reactions: Self::default_reactions(),
            poll: Self::default_poll(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for PostPopulatedGet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("PostPopulatedGet", 10)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("subject", &self.subject)?;
        state.serialize_field("body", &self.body)?;
        state.serialize_field("comment_ids", &self.comment_ids)?;
        state.serialize_field("reactions", &self.reactions)?;
        state.serialize_field("poll", &self.poll)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct PostPopulatedList {
    pub id: PostId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub subject: String,
    pub body: String,
    pub comment_ids: Vec<CommentId>,
    pub poll_id: Option<PollId>,
    pub _permission: ObjectPermission,
}

impl PostPopulatedList {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> PostId {
        <PostId as Default>::default().into()
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

    pub fn default_subject() -> String {
        <String as Default>::default().into()
    }

    pub fn default_body() -> String {
        <String as Default>::default().into()
    }

    pub fn default_comment_ids() -> Vec<CommentId> {
        <Vec<CommentId> as Default>::default().into()
    }

    pub fn default_poll_id() -> Option<PollId> {
        None
    }
}

sqlx_json_decode!(PostPopulatedList);

impl Default for PostPopulatedList {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            subject: Self::default_subject(),
            body: Self::default_body(),
            comment_ids: Self::default_comment_ids(),
            poll_id: Self::default_poll_id(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for PostPopulatedList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("PostPopulatedList", 9)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("subject", &self.subject)?;
        state.serialize_field("body", &self.body)?;
        state.serialize_field("comment_ids", &self.comment_ids)?;
        state.serialize_field("poll_id", &self.poll_id)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}
