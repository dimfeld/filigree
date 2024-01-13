use axum::{
    extract::{Host, Query, State},
    response::IntoResponse,
    Json,
};
use error_stack::ResultExt;
use filigree::auth::{
    passwordless_email_login::{
        check_signup_request, perform_passwordless_login, setup_passwordless_login,
    },
    LoginResult,
};
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::{
    models::user::{self, UserCreatePayload},
    server::ServerState,
    users::{
        organization::create_new_organization, users::create_new_user_with_prehashed_password,
    },
    Error,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePasswordlessLoginRequestBody {
    email: String,
    redirect_to: Option<String>,
}

pub async fn request_passwordless_login(
    State(state): State<ServerState>,
    Host(host): Host,
    Json(CreatePasswordlessLoginRequestBody { email, redirect_to }): Json<
        CreatePasswordlessLoginRequestBody,
    >,
) -> Result<impl IntoResponse, Error> {
    let token = setup_passwordless_login(&state.filigree, email.clone()).await;

    let token = match token {
        Ok(token) => token,
        Err(e) => {
            if e.current_context().is_unauthenticated() {
                // This means that the user does not exist and public signups are disabled.
                // Don't do anything in that case, but also don't tell the user that the email
                // doesn't exist.
                return Ok(());
            } else {
                return Err(e.change_context(Error::AuthSubsystem).into());
            }
        }
    };

    // TODO better validation against a list of allowed domains
    let redirect_to = if redirect_to
        .as_deref()
        .map(|s| s.contains("//"))
        .unwrap_or(false)
    {
        None
    } else {
        redirect_to
    };

    let template = crate::emails::PasswordlessLoginRequestTemplate {
        // TODO get the user's name in `setup_passwordless_login`, if we have it
        user_name: None,
        url_scheme: state.site_scheme(),
        // TODO validate that the host is in the allowed list
        host,
        email: email.clone(),
        redirect_to,
        token: token.token,
        invite: token.new_user,
    };

    state
        .filigree
        .email
        .send_template(email, template)
        .await
        .change_context(Error::AuthSubsystem)?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct PasswordlessLoginRequestQueryFromEmail {
    email: String,
    token: Uuid,
    redirect_to: Option<String>,
    #[serde(default)]
    invite: bool,
}

async fn accept_new_user_invite(
    state: &ServerState,
    cookies: &Cookies,
    email: String,
    token: Uuid,
) -> Result<(), error_stack::Report<Error>> {
    check_signup_request(state, &email, token)
        .await
        .change_context(Error::Login)?;

    let user_id = crate::models::user::UserId::new();
    let mut tx = state.db.begin().await.change_context(Error::Db)?;
    let created_org =
        create_new_organization(&mut *tx, "My Organization".to_string(), user_id).await?;

    let new_user = UserCreatePayload {
        email: email.clone(),
        ..Default::default()
    };

    create_new_user_with_prehashed_password(
        &mut *tx,
        user_id,
        created_org.organization.id,
        new_user,
        None,
    )
    .await?;

    filigree::users::users::add_user_email_login(&mut *tx, user_id, email, true)
        .await
        .change_context(Error::Db)?;

    tx.commit().await.change_context(Error::Db)?;

    state
        .filigree
        .session_backend
        .create_session(&cookies, &user_id)
        .await
        .change_context(Error::AuthSubsystem)?;

    Ok(())
}

pub async fn process_passwordless_login_token(
    State(state): State<ServerState>,
    cookies: Cookies,
    Query(q): Query<PasswordlessLoginRequestQueryFromEmail>,
) -> Result<impl IntoResponse, Error> {
    if q.invite {
        if !state.filigree.new_user_flags.allow_public_signup {
            return Err(Error::Login);
        }

        accept_new_user_invite(&state, &cookies, q.email.clone(), q.token).await?;
        // TODO Option to default redirect to special onboarding page here
    } else {
        perform_passwordless_login(&state.filigree, &cookies, q.email, q.token)
            .await
            .change_context(Error::Login)?;
    }

    let mut redirect_path = q.redirect_to.as_deref().unwrap_or("/");
    if redirect_path.contains("//") {
        // Very simple check to prevent redirects to other domains
        // This should actually validate against a whitelist of allowed domains
        redirect_path = "/";
    }

    Ok(Json(LoginResult {
        message: "Logged in".into(),
        redirect_to: Some(redirect_path.to_string()),
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        auth::tests::extract_token_from_email,
        tests::{start_app, BootstrappedData},
    };

    fn no_redirect_client() -> reqwest::Client {
        reqwest::ClientBuilder::new()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap()
    }

    #[sqlx::test]
    async fn passwordless_login_existing_user(db: sqlx::PgPool) {
        let (app, BootstrappedData { user, .. }) = start_app(db.clone()).await;

        let client = app.client.with_custom_client(no_redirect_client());

        client
            .post("auth/email_login")
            .json(&CreatePasswordlessLoginRequestBody {
                email: user.email.clone(),
                redirect_to: None,
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let email = app.sent_emails.lock().unwrap().pop().unwrap();

        assert!(email.html.contains("Click here to log in"));
        assert!(email.text.contains("/login?token="));
        assert!(!email.text.contains("&invite=true"));

        let token = extract_token_from_email(&email);

        let url = format!(
            "auth/email_login?token={token}&email={email}",
            email = user.email
        );
        client
            .get(&url)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let response: serde_json::Value = client
            .get("self")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(response["email"], user.email);
        assert_eq!(response["name"], "User");

        // Using the token again should fail.
        let reuse_response = client.get(&url).send().await.unwrap();
        assert_eq!(reuse_response.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    #[ignore = "todo"]
    async fn passwordless_login_bad_tokens(db: sqlx::PgPool) {
        let (app, BootstrappedData { user, .. }) = start_app(db.clone()).await;

        println!("== Test expired token");
        app.client
            .post("auth/email_login")
            .json(&CreatePasswordlessLoginRequestBody {
                email: user.email.clone(),
                redirect_to: None,
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let email = app.sent_emails.lock().unwrap().pop().unwrap();
        let token = extract_token_from_email(&email);

        // Force the token to be expired
        sqlx::query!(
            r##"UPDATE email_logins
            SET passwordless_login_expires_at = now() - '1 second'::interval
            WHERE email = $1"##,
            user.email
        )
        .execute(&db)
        .await
        .unwrap();

        let url = format!(
            "auth/email_login?token={token}&email={email}",
            email = user.email
        );
        let response = app.client.get(&url).send().await.unwrap();
        assert_eq!(
            response.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "Expired token"
        );

        // generate a token again
        app.client
            .post("auth/email_login")
            .json(&CreatePasswordlessLoginRequestBody {
                email: user.email.clone(),
                redirect_to: None,
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        println!("== Test invalid token");
        let url = format!(
            "auth/email_login?token={bad_token}&email={user_email}",
            bad_token = Uuid::new_v4(),
            user_email = user.email
        );

        let response = app.client.get(&url).send().await.unwrap();
        assert_eq!(
            response.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "Invalid token"
        );

        println!("== Valid token should be wiped after first bad attempt");
        let email = app.sent_emails.lock().unwrap().pop().unwrap();
        let token = extract_token_from_email(&email);
        let response = app
            .client
            .get(&format!(
                "auth/email_login?token={token}&email={user_email}",
                user_email = user.email
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            reqwest::StatusCode::UNAUTHORIZED,
            "Valid token should have been wiped after previous bad attempt"
        );
    }

    #[sqlx::test]
    async fn passwordless_login_new_user(db: sqlx::PgPool) {
        // TODO This assumes public_sign_up is enabled.
        let (app, BootstrappedData { organization, .. }) = start_app(db.clone()).await;

        let client = app.client.clone();

        let new_user_email = "new@new_user.com".to_string();

        app.client
            .post("auth/email_login")
            .json(&CreatePasswordlessLoginRequestBody {
                email: new_user_email.clone(),
                redirect_to: None,
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let email = app.sent_emails.lock().unwrap().pop().unwrap();
        let token = extract_token_from_email(&email);
        assert!(email.text.contains("&invite=true"));

        client
            .get(&format!(
                "auth/email_login?token={token}&email={new_user_email}&invite=true",
            ))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();

        let response: serde_json::Value = client
            .get("self")
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(response["email"], new_user_email);

        let user_org = sqlx::query!(
            "SELECT organization_id FROM users WHERE email = $1",
            new_user_email
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert!(
            &user_org.organization_id != organization.id.as_uuid(),
            "User should be in a new organization"
        );
    }
}
