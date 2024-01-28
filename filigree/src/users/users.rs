use async_trait::async_trait;
use sqlx::PgExecutor;
use url::Url;

use crate::auth::{OrganizationId, UserId};

/// Add a new user email login mapping. If `preverfied` is false, the verification token will be
/// returned.
pub async fn add_user_email_login(
    tx: impl PgExecutor<'_>,
    user_id: UserId,
    email: String,
    preverified: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO email_logins (user_id, email, verified)
       VALUES ($1, $2, $3)",
        user_id.as_uuid(),
        email,
        preverified,
    )
    .execute(tx)
    .await?;

    Ok(())
}

/// User details that may be present when first creating a user, without any information specific
/// to your application. This information may come from an initial signup, an email invite, or an OAuth login.
pub struct CreateUserDetails {
    /// The user's name
    pub name: Option<String>,
    /// The user's email
    pub email: Option<String>,
    /// An avatar image for this user
    pub avatar_url: Option<Url>,
}

/// Allow filigree to call into the database to create a new user, along with all the appropriate
/// related information.
///
/// If `add_to_organization` is provided, the user should be created in that organization. If omitted, the
/// default behavior should be done, which will usually be creating a new organization or placing
/// the user into a "global" organization.
#[async_trait]
pub trait UserCreator: Send + Sync + 'static {
    /// Create a new user
    async fn create_user(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        add_to_organization: Option<OrganizationId>,
        details: &CreateUserDetails,
    ) -> Result<UserId, sqlx::Error>;
}
