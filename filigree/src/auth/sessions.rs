use std::{fmt::Display, str::FromStr};

use error_stack::{Report, ResultExt};
use sqlx::PgPool;
use thiserror::Error;
use tower_cookies::Cookie;
use uuid::Uuid;

use super::{OrganizationId, UserId};

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
    pub fn expiry_time(&self) -> std::time::Duration {
        match self {
            ExpiryStyle::FromCreation(duration) => *duration,
            ExpiryStyle::AfterIdle(duration) => *duration,
        }
    }
}

pub struct SessionManager {
    db: PgPool,
    cookies: SessionCookieBuilder,
    expiry_style: ExpiryStyle,
}

pub struct SessionKey {
    session_id: SessionId,
    user_id: UserId,
}

impl SessionKey {
    pub fn new(user_id: UserId) -> Self {
        Self::new_from_id(SessionId::new(), user_id)
    }

    pub fn new_from_id(session_id: SessionId, user_id: UserId) -> Self {
        Self {
            session_id,
            user_id,
        }
    }
}

impl Display for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.session_id, self.user_id)
    }
}

impl FromStr for SessionKey {
    type Err = SessionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id, user_id) = s.split_once(':').ok_or_else(|| SessionError::NotFound)?;
        let id = SessionId::from_str(id).map_err(|_| SessionError::NotFound)?;
        let user_id = UserId::from_str(user_id).map_err(|_| SessionError::NotFound)?;

        Ok(Self::new_from_id(id, user_id))
    }
}

impl SessionManager {
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

    pub async fn add_session(
        &self,
        user_id: &UserId,
        org_id: &OrganizationId,
    ) -> Result<Cookie, Report<SessionError>> {
        let session_id = SessionId::new();

        sqlx::query!(
            "INSERT INTO user_sessions (id, user_id, organization_id, expires_at) VALUES ($1, $2, $3, now() + $4)",
            &session_id,
            &user_id,
            &org_id,
            self.expiry_style.expiry_time()
        )
        .execute(&self.db)
        .await
        .change_context(SessionError::Db)?;

        Ok(self.cookies.create_cookie(
            &SessionKey::new_from_id(session_id, *user_id),
            self.expiry_style.expiry_time(),
        ))
    }

    pub async fn touch_session(
        &self,
        id: SessionId,
        user_id: UserId,
    ) -> Result<Option<Cookie>, SessionError> {
        let ExpiryStyle::AfterIdle(duration) = self.expiry_style else {
            return Ok(None);
        };

        let updated = sqlx::query!(
            "UPDATE user_sessions
                SET expires_at = now() + $1
                WHERE id=$2 and user_id=$3
                -- Prevent unnecessary updates
                AND expires_at < now() + $1 - '1 minute",
            duration,
            id,
            user_id
        )
        .execute(&self.db)
        .await
        .change_context(SessionError::Db)?;

        if updated > 0 {
            Ok(Some(self.cookies.create_cookie(
                &SessionKey::new_from_id(id, user_id),
                duration,
            )))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), Report<SessionError>> {
        sqlx::query!("DELETE FROM user_sessions WHERE id = $1", id)
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)
    }

    pub async fn delete_expired_sessions(&self) -> Result<(), Report<SessionError>> {
        sqlx::query!("DELETE FROM user_sessions WHERE expires_at < now()")
            .execute(&self.db)
            .await
            .change_context(SessionError::Db)
    }
}
