use std::{marker::PhantomData, ops::Deref, str::FromStr};

use base64::{display::Base64Display, engine::GeneralPurpose, Engine};
use thiserror::Error;
use uuid::Uuid;

#[macro_export]
macro_rules! make_object_id {
    ($typ:ident, $prefix:ident) => {
        mod $prefix {
            pub struct $typ;
            impl $crate::object_id::ObjectIdPrefix for $typ {
                fn prefix() -> &'static str {
                    stringify!($prefix)
                }
            }
        }

        pub type $typ = ObjectId<$prefix::$typ>;
    };
}

#[derive(Debug, Error)]
pub enum ObjectIdError {
    #[error("Invalid ID prefix, expected {0}")]
    InvalidPrefix(&'static str),

    #[error("Failed to decode object ID")]
    DecodeFailure,
}

pub trait ObjectIdPrefix {
    fn prefix() -> &'static str;
}

/// A type that is internally stored as a UUID but externally as a
/// more accessible string with a prefix indicating its type. This uses
/// UUID v7 so that the output will be lexicographically sortable.
#[derive(Copy, Clone, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId<PREFIX: ObjectIdPrefix>(pub Uuid, PhantomData<PREFIX>);

impl<PREFIX: ObjectIdPrefix> ObjectId<PREFIX> {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7(), PhantomData::default())
    }

    pub fn from_uuid(u: Uuid) -> Self {
        Self(u, PhantomData::default())
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn nil() -> Self {
        Self(Uuid::nil(), PhantomData::default())
    }

    pub fn display_without_prefix(&self) -> Base64Display<GeneralPurpose> {
        base64::display::Base64Display::new(
            self.0.as_bytes(),
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        )
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

impl<PREFIX: ObjectIdPrefix> Deref for ObjectId<PREFIX> {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<PREFIX: ObjectIdPrefix> From<Uuid> for ObjectId<PREFIX> {
    fn from(u: Uuid) -> Self {
        Self(u, PhantomData::default())
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

pub fn decode_suffix(s: &str) -> Result<Uuid, ObjectIdError> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
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

#[cfg(test)]
mod tests {
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
        assert_eq!(id, id2, "Value serializes and deserializes to itself");
    }
}
