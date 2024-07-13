use std::{marker::PhantomData, str::FromStr};

use base64::{
    display::Base64Display,
    engine::{general_purpose::URL_SAFE_NO_PAD, GeneralPurpose},
    Engine,
};
use schemars::JsonSchema;
use sea_orm::IntoActiveValue;
use sqlx::{postgres::PgTypeInfo, Database};
use thiserror::Error;
use uuid::Uuid;

/// Create a new ObjectId type. This automatically implements the prefix structure and creates
/// a type alias for the type.
#[macro_export]
macro_rules! make_object_id {
    ($typ:ident, $prefix:ident) => {
        mod $prefix {
            #[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
            pub struct $typ;
            impl $crate::object_id::ObjectIdPrefix for $typ {
                fn prefix() -> &'static str {
                    stringify!($prefix)
                }
            }
        }

        /// The ObjectId type alias for this model.
        pub type $typ = $crate::object_id::ObjectId<$prefix::$typ>;
    };
}

/// An error related to parsing an ObjectId
#[derive(Debug, Error)]
pub enum ObjectIdError {
    /// The prefix in the parsed ID did not match the expected prefix
    #[error("Invalid ID prefix, expected {0}")]
    InvalidPrefix(&'static str),

    /// Some other parsing error, such as invalid base64
    #[error("Failed to decode object ID")]
    DecodeFailure,
}

/// An object that provides a the prefix for a serialized ObjectId.
pub trait ObjectIdPrefix:
    Clone + Copy + Eq + PartialEq + PartialOrd + Ord + std::hash::Hash
{
    /// The short prefix for this ID type
    fn prefix() -> &'static str;
}

/// A type that is internally stored as a UUID but externally as a
/// more accessible string with a prefix indicating its type. This uses
/// UUID v7 so that the output will be lexicographically sortable.
#[derive(Hash, PartialOrd, Ord, Eq)]
pub struct ObjectId<PREFIX: ObjectIdPrefix>(pub Uuid, PhantomData<PREFIX>);

impl<PREFIX: ObjectIdPrefix> JsonSchema for ObjectId<PREFIX> {
    fn schema_name() -> String {
        format!("ObjectId<{}>", PREFIX::prefix())
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }

    fn is_referenceable() -> bool {
        false
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!(concat!(module_path!(), "::ObjectId<{}>"), PREFIX::prefix()).into()
    }
}

impl<PREFIX: ObjectIdPrefix> Clone for ObjectId<PREFIX> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<PREFIX: ObjectIdPrefix> Copy for ObjectId<PREFIX> {}

impl<PREFIX: ObjectIdPrefix> ObjectId<PREFIX> {
    /// Create a new ObjectId with a timestamp of now
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7(), PhantomData)
    }

    /// Create a new ObjectId from a UUID
    pub const fn from_uuid(u: Uuid) -> Self {
        Self(u, PhantomData)
    }

    /// Return the inner Uuid
    pub const fn into_inner(self) -> Uuid {
        self.0
    }

    /// Return a reference to the inner Uuid
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Return an ObjectId corresponding to the "all zeroes" UUID
    pub const fn nil() -> Self {
        Self(Uuid::nil(), PhantomData)
    }

    /// Writes the UUID portion of the object ID, without the prefix
    pub fn display_without_prefix(&self) -> Base64Display<GeneralPurpose> {
        base64::display::Base64Display::new(self.0.as_bytes(), &URL_SAFE_NO_PAD)
    }
}

impl<PREFIX: ObjectIdPrefix> Default for ObjectId<PREFIX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<PREFIX: ObjectIdPrefix> PartialEq for ObjectId<PREFIX> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<PREFIX: ObjectIdPrefix> PartialEq<Uuid> for ObjectId<PREFIX> {
    fn eq(&self, other: &Uuid) -> bool {
        &self.0 == other
    }
}

impl<PREFIX: ObjectIdPrefix> AsRef<Uuid> for ObjectId<PREFIX> {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl<PREFIX: ObjectIdPrefix> From<Uuid> for ObjectId<PREFIX> {
    fn from(u: Uuid) -> Self {
        Self(u, PhantomData)
    }
}

impl<PREFIX: ObjectIdPrefix> From<ObjectId<PREFIX>> for Uuid {
    fn from(data: ObjectId<PREFIX>) -> Self {
        data.0
    }
}

impl<PREFIX: ObjectIdPrefix> std::fmt::Debug for ObjectId<PREFIX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectId")
            .field(&self.to_string())
            .field(&self.0)
            .finish()
    }
}

impl<PREFIX: ObjectIdPrefix> std::fmt::Display for ObjectId<PREFIX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(PREFIX::prefix())?;
        self.display_without_prefix().fmt(f)
    }
}

fn decode_suffix(s: &str) -> Result<Uuid, ObjectIdError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| ObjectIdError::DecodeFailure)?;
    Uuid::from_slice(&bytes).map_err(|_| ObjectIdError::DecodeFailure)
}

impl<PREFIX: ObjectIdPrefix> FromStr for ObjectId<PREFIX> {
    type Err = ObjectIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expected_prefix = PREFIX::prefix();
        if !s.starts_with(expected_prefix) {
            return Err(ObjectIdError::InvalidPrefix(expected_prefix));
        }

        decode_suffix(&s[expected_prefix.len()..]).map(Self::from_uuid)
    }
}

/// Serialize into string form with the prefix
impl<PREFIX: ObjectIdPrefix> serde::Serialize for ObjectId<PREFIX> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

struct ObjectIdVisitor<PREFIX: ObjectIdPrefix>(PhantomData<PREFIX>);

impl<PREFIX: ObjectIdPrefix> Default for ObjectIdVisitor<PREFIX> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<'de, PREFIX: ObjectIdPrefix> serde::de::Visitor<'de> for ObjectIdVisitor<PREFIX> {
    type Value = ObjectId<PREFIX>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an object ID starting with ")?;
        formatter.write_str(PREFIX::prefix())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match Self::Value::from_str(v) {
            Ok(id) => Ok(id),
            Err(e) => {
                // See if it's in UUID format instead of the encoded format. This mostly happens when
                // deserializing from a JSON object generated in Postgres with jsonb_build_object.
                Uuid::from_str(v)
                    .map(ObjectId::<PREFIX>::from_uuid)
                    // Return the more descriptive original error instead of the UUID parsing error
                    .map_err(|_| e)
            }
        }
        .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(v), &self))
    }
}

/// Deserialize from string form with the prefix.
impl<'de, PREFIX: ObjectIdPrefix> serde::Deserialize<'de> for ObjectId<PREFIX> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ObjectIdVisitor::default())
    }
}

/// Store and retrieve in Postgres as a raw UUID
impl<PREFIX: ObjectIdPrefix> sqlx::Type<sqlx::Postgres> for ObjectId<PREFIX> {
    fn type_info() -> <sqlx::Postgres as Database>::TypeInfo {
        <sqlx::types::Uuid as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<PREFIX: ObjectIdPrefix> sqlx::postgres::PgHasArrayType for ObjectId<PREFIX> {
    fn array_type_info() -> PgTypeInfo {
        <sqlx::types::Uuid as sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

impl<'q, PREFIX: ObjectIdPrefix> sqlx::Encode<'q, sqlx::Postgres> for ObjectId<PREFIX> {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Postgres as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        <sqlx::types::Uuid as sqlx::Encode<'_, sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

impl<'r, PREFIX: ObjectIdPrefix> sqlx::Decode<'r, sqlx::Postgres> for ObjectId<PREFIX> {
    fn decode(
        value: <sqlx::Postgres as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let u = <sqlx::types::Uuid as sqlx::Decode<'r, sqlx::Postgres>>::decode(value)?;
        Ok(Self(u, PhantomData))
    }
}

// SeaORM traits
impl<PREFIX: ObjectIdPrefix> From<ObjectId<PREFIX>> for sea_orm::Value {
    fn from(value: ObjectId<PREFIX>) -> Self {
        value.0.into()
    }
}

impl<PREFIX: ObjectIdPrefix> IntoActiveValue<ObjectId<PREFIX>> for ObjectId<PREFIX> {
    fn into_active_value(self) -> sea_orm::ActiveValue<ObjectId<PREFIX>> {
        sea_orm::ActiveValue::Set(self)
    }
}

impl<PREFIX: ObjectIdPrefix> sea_orm::TryGetable for ObjectId<PREFIX> {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        <Uuid as sea_orm::TryGetable>::try_get_by(res, idx).map(ObjectId::from)
    }
}

impl<PREFIX: ObjectIdPrefix> sea_orm::sea_query::ValueType for ObjectId<PREFIX> {
    fn try_from(v: sea_orm::Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <Uuid as sea_orm::sea_query::ValueType>::try_from(v).map(ObjectId::from)
    }

    fn type_name() -> String {
        stringify!(ObjectId).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::Uuid
    }

    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::sea_query::ColumnType::Uuid
    }
}

impl<PREFIX: ObjectIdPrefix> sea_orm::sea_query::Nullable for ObjectId<PREFIX> {
    fn null() -> sea_orm::Value {
        sea_orm::Value::Uuid(None)
    }
}

impl<PREFIX: ObjectIdPrefix> sea_orm::TryFromU64 for ObjectId<PREFIX> {
    fn try_from_u64(_v: u64) -> Result<Self, sea_orm::DbErr> {
        Err(sea_orm::DbErr::ConvertFromU64("uuid"))
    }
}

#[cfg(test)]
mod tests {
    use axum::{extract::Path, response::IntoResponse, Router};

    use super::*;

    make_object_id!(TeamId, tm);

    #[test]
    fn to_from_str() {
        let id = TeamId::new();

        let s = id.to_string();
        let id2 = TeamId::from_str(&s).unwrap();
        assert_eq!(id, id2, "ID converts to string and back");
    }

    #[test]
    fn serde() {
        let id = TeamId::new();
        let json_str = serde_json::to_string(&id).unwrap();
        let id2: TeamId = serde_json::from_str(&json_str).unwrap();
        drop(json_str);
        assert_eq!(id, id2, "Value serializes and deserializes to itself");
    }

    #[test]
    fn can_use_in_axum_path() {
        async fn get_id(Path(_id): Path<TeamId>) -> impl IntoResponse {
            "ok"
        }

        let _ = Router::<()>::new().route("/:id", axum::routing::get(get_id));
    }
}
