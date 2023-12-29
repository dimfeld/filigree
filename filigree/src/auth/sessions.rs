use std::{fmt::Display, str::FromStr};

use axum::http::request::Parts;
use error_stack::{Report, ResultExt};
use sqlx::PgPool;
use thiserror::Error;
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

use super::UserId;

crate::make_object_id!(SessionId, sid);

/// Errors when creating or retrieving a session
#[derive(Debug, Error)]
pub enum SessionError {
    /// An error accessing the database
    #[error("Failed to access database")]
    Db,
    /// Failed to find a session in the database
    #[error("Session does not exist")]
    NotFound,
}

/// Builds cookies and stores some settings that will apply to all generated cookies.
pub struct SessionCookieBuilder {
    secure: bool,
    same_site: tower_cookies::cookie::SameSite,
}

impl SessionCookieBuilder {
    /// Create a new [SessionCookieBuilder]
    pub fn new(secure: bool, same_site: tower_cookies::cookie::SameSite) -> Self {
        Self { secure, same_site }
    }

    /// Create a session cookie
    pub fn create_cookie(&self, key: &SessionKey, expiry: std::time::Duration) -> Cookie<'static> {
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

/// How session expiration should be calculated
pub enum ExpiryStyle {
    /// Always expire the session at a fixed duration after it is created
    FromCreation(std::time::Duration),
    /// Expire the session after no activity is seen for the given duration
    AfterIdle(std::time::Duration),
}

impl ExpiryStyle {
    /// Return the expiry duration, regardless of the style
    pub fn expiry_duration(&self) -> std::time::Duration {
        match self {
            ExpiryStyle::FromCreation(duration) => *duration,
            ExpiryStyle::AfterIdle(duration) => *duration,
        }
    }
}

/// The backend for storing and retrieving session information.
pub struct SessionBackend {
    db: PgPool,
    cookies: SessionCookieBuilder,
    expiry_style: ExpiryStyle,
}

/// The cookie value for a session, parsed into its individual values
pub struct SessionKey {
    /// The id for the session
    pub session_id: SessionId,
    /// A random UUID to make it slightly harder to guess a valid session key.
    /// This is somewhat overkill since the ID is already a UUID.
    pub hash: Uuid,
}

impl SessionKey {
    /// Create a new session key
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
    /// Create the [SessionBackend]
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

    /// Create a new session and set a cookie with the session key.
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
            session_id.as_uuid(),
            &hash,
            user_id.as_uuid(),
            self.expiry_style.expiry_duration() as _
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
    ) -> Result<(), Report<SessionError>> {
        let ExpiryStyle::AfterIdle(duration) = self.expiry_style else {
            return Ok(());
        };

        let updated = sqlx::query!(
            "UPDATE user_sessions
                SET expires_at = now() + $1
                WHERE id=$2 and hash=$3
                -- Prevent unnecessary updates
                AND (expires_at < now() + $1 - '1 minute'::interval)",
            duration as _,
            &key.session_id as _,
            &key.hash
        )
        .execute(&self.db)
        .await
        .change_context(SessionError::Db)?;

        if updated.rows_affected() > 0 {
            cookies.add(self.cookies.create_cookie(&key, duration));
        }

        Ok(())
    }

    /// Delete all sessions for a user
    pub async fn delete_for_user(&self, id: UserId) -> Result<(), Report<SessionError>> {
        sqlx::query!("DELETE FROM user_sessions WHERE user_id = $1", id.as_uuid())
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)?;
        Ok(())
    }

    /// Delete a session, as when logging out.
    pub async fn delete_session(
        &self,
        cookies: &Cookies,
        id: &str,
    ) -> Result<(), Report<SessionError>> {
        cookies.remove(Cookie::new("sid", ""));

        sqlx::query!("DELETE FROM user_sessions WHERE id = $1", id as _)
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)?;
        Ok(())
    }

    /// Sweep the session table and remove any expired sessions.
    pub async fn delete_expired_sessions(&self) -> Result<(), Report<SessionError>> {
        sqlx::query!("DELETE FROM user_sessions WHERE expires_at < now()")
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)?;
        Ok(())
    }
}

/// Try to retrieve the session cookie from the request [Parts]
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
        let key = SessionKey::new(sid.clone(), hash);

        let serialized = key.to_string();

        let restored = SessionKey::from_str(&serialized).unwrap();
        assert_eq!(restored.session_id, sid);
        assert_eq!(restored.hash, hash);
    }
}
