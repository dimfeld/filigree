use std::{fmt::Display, str::FromStr};

use axum::{extract::Request, http::request::Parts};
use error_stack::{Report, ResultExt};
use sqlx::PgPool;
use thiserror::Error;
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

use super::UserId;

crate::make_object_id!(SessionId, sid);

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to access database")]
    Db,
    #[error("Session does not exist")]
    NotFound,
}

pub struct SessionCookieBuilder {
    secure: bool,
    same_site: tower_cookies::cookie::SameSite,
}

impl SessionCookieBuilder {
    /// Create a new `SessionCookieBuilder`
    pub fn new(secure: bool, same_site: tower_cookies::cookie::SameSite) -> Self {
        Self { secure, same_site }
    }

    /// Create a session cookie
    pub fn create_cookie(&self, key: &SessionKey, expiry: std::time::Duration) -> Cookie {
        let cookie_contents = key.to_string();
        let expiry = tower_cookies::cookie::time::Duration::try_from(expiry).unwrap();
        Cookie::build(("sid", cookie_contents))
            .http_only(true)
            .same_site(self.same_site)
            .secure(self.secure)
            .max_age(expiry)
            .path("/")
            .into()
    }
}

pub enum ExpiryStyle {
    FromCreation(std::time::Duration),
    AfterIdle(std::time::Duration),
}

impl ExpiryStyle {
    pub fn expiry_duration(&self) -> std::time::Duration {
        match self {
            ExpiryStyle::FromCreation(duration) => *duration,
            ExpiryStyle::AfterIdle(duration) => *duration,
        }
    }
}

pub struct SessionBackend {
    db: PgPool,
    cookies: SessionCookieBuilder,
    expiry_style: ExpiryStyle,
}

pub struct SessionKey {
    pub session_id: SessionId,
    pub hash: Uuid,
}

impl SessionKey {
    pub fn new(session_id: SessionId, hash: Uuid) -> Self {
        Self { session_id, hash }
    }
}

impl Display for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.session_id, self.hash)
    }
}

impl FromStr for SessionKey {
    type Err = SessionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id, hash) = s.split_once(':').ok_or(SessionError::NotFound)?;
        let id = SessionId::from_str(id).map_err(|_| SessionError::NotFound)?;
        let hash = Uuid::from_str(hash).map_err(|_| SessionError::NotFound)?;

        Ok(Self::new(id, hash))
    }
}

impl SessionBackend {
    pub fn new(db: PgPool, cookies: SessionCookieBuilder, expiry_style: ExpiryStyle) -> Self {
        Self {
            db,
            cookies,
            expiry_style,
        }
    }

    // pub async fn get_session(&self, session_key: &str) -> Result<(), Report<SessionError>> {
    //     let key = SessionKey::from_str(session_key)?;

    //     sqlx::query_as!(
    //         SessionContents,
    //         "SELECT id, user_id, org_id, created_at, expires_at FROM user_sessions WHERE id = $1 and user_id = $2 AND expires_at > now()",
    //         id,
    //         user_id
    //     )
    //     .fetch_optional(&self.db)
    //     .await
    //     .change_context(SessionError::Db)?
    //     .ok_or(SessionError::NotFound)?;

    //     Ok(key)
    // }

    pub async fn create_session(
        &self,
        cookies: &Cookies,
        user_id: &UserId,
    ) -> Result<(), Report<SessionError>> {
        let session_id = SessionId::new();
        let hash = Uuid::new_v4();

        sqlx::query!(
            "
            INSERT INTO user_sessions (id, user_id, hash, expires_at) VALUES
            ($1, $2, $3, now() + $4)",
            &session_id,
            &hash,
            &user_id,
            self.expiry_style.expiry_time()
        )
        .execute(&self.db)
        .await
        .change_context(SessionError::Db)?;

        let cookie = self.cookies.create_cookie(
            &SessionKey::new(session_id, hash),
            self.expiry_style.expiry_duration(),
        );

        cookies.add(cookie);
        Ok(())
    }

    /// Update a session with the new expiry time. This usually is not called directly since it is
    /// part of the query that retrieves the actual user as well.
    pub async fn touch_session(
        &self,
        cookies: &Cookies,
        key: &SessionKey,
    ) -> Result<(), SessionError> {
        let ExpiryStyle::AfterIdle(duration) = self.expiry_style else {
            return Ok(());
        };

        let updated = sqlx::query!(
            "UPDATE user_sessions
                SET expires_at = now() + $1
                WHERE id=$2 and hash=$3
                -- Prevent unnecessary updates
                AND (expires_at < now() + $1 - '1 minute')",
            duration,
            &key.id,
            &key.hash
        )
        .execute(&self.db)
        .await
        .change_context(SessionError::Db)?;

        if updated > 0 {
            cookies.add(self.cookies.create_cookie(&key, duration));
        }

        Ok(())
    }

    /// Delete all sessions for a user
    pub async fn delete_for_user(&self, id: UserId) -> Result<(), Report<SessionError>> {
        sqlx::query!("DELETE FROM user_sessions WHERE user_id = $1", id)
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)
    }

    /// Delete a session, as when logging out.
    pub async fn delete_session(
        &self,
        cookies: &Cookies,
        id: &str,
    ) -> Result<(), Report<SessionError>> {
        cookies.remove(Cookie::new("sid", ""));

        sqlx::query!("DELETE FROM user_sessions WHERE id = $1", id)
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)
    }

    /// Sweep the session table and remove any expired sessions.
    pub async fn delete_expired_sessions(&self) -> Result<(), Report<SessionError>> {
        sqlx::query!("DELETE FROM user_sessions WHERE expires_at < now()")
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)
    }
}

pub fn get_session_cookie(request: &Parts) -> Option<SessionKey> {
    let cookies = request.extensions.get::<Cookies>()?;
    let sid = cookies.get("sid")?;
    SessionKey::from_str(sid.value()).ok()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn session_key() {
        let sid = SessionId::new();
        let hash = Uuid::new_v4();
        let key = SessionKey::new(sid, hash);

        let serialized = key.to_string();

        let restored = SessionKey::from_str(&serialized).unwrap();
        assert_eq!(restored.session_id, sid);
        assert_eq!(restored.hash, hash);
    }
}