#![allow(unused_imports, dead_code)]
use filigree::auth::ObjectPermission;
use serde::{
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use sqlx_transparent_json_decode::sqlx_json_decode;

use super::PollId;
use crate::models::{organization::OrganizationId, post::PostId};

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]

pub struct Poll {
    pub id: PollId,
    pub organization_id: crate::models::organization::OrganizationId,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub question: String,
    pub answers: serde_json::Value,
    pub post_id: PostId,
    pub _permission: ObjectPermission,
}

pub type PollPopulatedGet = Poll;

pub type PollPopulatedList = Poll;

pub type PollCreateResult = Poll;

impl Poll {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> PollId {
        <PollId as Default>::default().into()
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

    pub fn default_question() -> String {
        <String as Default>::default().into()
    }

    pub fn default_answers() -> serde_json::Value {
        <serde_json::Value as Default>::default().into()
    }

    pub fn default_post_id() -> PostId {
        <PostId as Default>::default().into()
    }
}

sqlx_json_decode!(Poll);

impl Default for Poll {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            organization_id: Self::default_organization_id(),
            updated_at: Self::default_updated_at(),
            created_at: Self::default_created_at(),
            question: Self::default_question(),
            answers: Self::default_answers(),
            post_id: Self::default_post_id(),
            _permission: ObjectPermission::Owner,
        }
    }
}

impl Serialize for Poll {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Poll", 8)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("organization_id", &self.organization_id)?;
        state.serialize_field("updated_at", &self.updated_at)?;
        state.serialize_field("created_at", &self.created_at)?;
        state.serialize_field("question", &self.question)?;
        state.serialize_field("answers", &self.answers)?;
        state.serialize_field("post_id", &self.post_id)?;
        state.serialize_field("_permission", &self._permission)?;
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone, schemars::JsonSchema, sqlx::FromRow)]
#[cfg_attr(test, derive(Serialize))]
pub struct PollCreatePayloadAndUpdatePayload {
    pub id: Option<PollId>,
    pub question: String,
    pub answers: serde_json::Value,
    pub post_id: PostId,
}

pub type PollCreatePayload = PollCreatePayloadAndUpdatePayload;

pub type PollUpdatePayload = PollCreatePayloadAndUpdatePayload;

impl PollCreatePayloadAndUpdatePayload {
    // The <T as Default> syntax here is weird but lets us generate from the template without needing to
    // detect whether to add the extra :: in cases like DateTime::<Utc>::default

    pub fn default_id() -> Option<PollId> {
        None
    }

    pub fn default_question() -> String {
        <String as Default>::default().into()
    }

    pub fn default_answers() -> serde_json::Value {
        <serde_json::Value as Default>::default().into()
    }

    pub fn default_post_id() -> PostId {
        <PostId as Default>::default().into()
    }
}

impl Default for PollCreatePayloadAndUpdatePayload {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            question: Self::default_question(),
            answers: Self::default_answers(),
            post_id: Self::default_post_id(),
        }
    }
}
