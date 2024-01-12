use std::borrow::Cow;

use async_trait::async_trait;
use axum::routing::{self, Router};
use filigree::auth::{AuthError, OrganizationId, PermissionChecker, RoleId, SessionKey, UserId};
use sqlx::{query_file_as, PgPool};
use uuid::Uuid;

use crate::server::ServerState;

pub mod password_management;
pub mod passwordless_login;
pub mod permissions;
#[cfg(test)]
mod tests;

pub type Authed = filigree::auth::Authed<AuthInfo>;

#[derive(Debug, sqlx::FromRow)]
pub struct AuthInfo {
    pub user_id: UserId,
    pub organization_id: OrganizationId,
    pub active: bool,
    pub verified: bool,
    pub roles: Vec<RoleId>,
    pub permissions: Vec<String>,
}

impl AuthInfo {
    pub fn actor_ids(&self) -> Vec<Uuid> {
        self.roles
            .iter()
            .map(|id| *id.as_uuid())
            .chain(std::iter::once(*self.user_id.as_uuid()))
            .collect::<Vec<_>>()
    }
}

impl filigree::auth::AuthInfo for AuthInfo {
    fn check_valid(&self) -> Result<(), AuthError> {
        if !self.active {
            Err(AuthError::Disabled)
        } else if !self.verified {
            Err(AuthError::NotVerified)
        } else {
            Ok(())
        }
    }

    fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }
}

pub struct AuthQueries {
    db: PgPool,
}

impl AuthQueries {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl filigree::auth::AuthQueries for AuthQueries {
    type AuthInfo = AuthInfo;

    async fn get_user_by_api_key(
        &self,
        api_key: Uuid,
        hash: Vec<u8>,
    ) -> Result<Option<AuthInfo>, sqlx::Error> {
        query_file_as!(AuthInfo, "src/auth/fetch_api_key.sql", api_key, hash)
            .fetch_optional(&self.db)
            .await
    }

    async fn get_user_by_session_id(
        &self,
        session_key: &SessionKey,
    ) -> Result<Option<AuthInfo>, sqlx::Error> {
        query_file_as!(
            AuthInfo,
            "src/auth/fetch_session.sql",
            session_key.session_id.as_uuid(),
            &session_key.hash
        )
        .fetch_optional(&self.db)
        .await
    }
}

pub fn has_permission(
    permission: impl Into<Cow<'static, str>>,
) -> filigree::auth::HasPermissionLayer<AuthInfo, impl PermissionChecker<AuthInfo>> {
    filigree::auth::has_permission(permission.into())
}

pub fn has_any_permission(
    permissions: Vec<impl Into<Cow<'static, str>>>,
) -> filigree::auth::HasPermissionLayer<AuthInfo, impl PermissionChecker<AuthInfo>> {
    filigree::auth::has_any_permission(permissions)
}

pub fn has_all_permissions(
    permissions: Vec<impl Into<Cow<'static, str>>>,
) -> filigree::auth::HasPermissionLayer<AuthInfo, impl PermissionChecker<AuthInfo>> {
    filigree::auth::has_all_permissions(permissions)
}

pub fn has_auth_predicate<F>(
    message: impl Into<Cow<'static, str>>,
    f: F,
) -> filigree::auth::HasPredicateLayer<AuthInfo, F>
where
    F: Fn(&AuthInfo) -> bool + Clone,
{
    filigree::auth::has_auth_predicate(message.into(), f)
}

pub fn create_routes() -> Router<ServerState> {
    Router::new()
        .route(
            "/auth/email_login",
            routing::post(passwordless_login::request_passwordless_login),
        )
        .route(
            "/auth/email_login",
            routing::get(passwordless_login::process_passwordless_login_token),
        )
        .route(
            "/auth/request_password_reset",
            routing::post(password_management::start_password_reset),
        )
}
