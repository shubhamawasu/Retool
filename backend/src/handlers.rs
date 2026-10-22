use crate::backend::{AuthSession, OidcCredentials}; // see note below on Credentials type
use crate::providers::IdentityProvider;
use crate::providers::mock::MockProvider;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tower_sessions::Session;

pub async fn login_handler<I: IdentityProvider + 'static>(
    State(provider): State<Arc<I>>,
    session: Session,
) -> impl IntoResponse {
    let auth = provider.authorize_url();

    session.insert("csrf_token", &auth.csrf_token()).await.ok();
    session.insert("nonce", &auth.nonce()).await.ok();

    Redirect::to(&auth.url())
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

#[axum::debug_handler]
pub async fn concrete_callback(
    state: State<Arc<MockProvider>>,
    auth_session: AuthSession,
    session: Session,
    query: Query<CallbackParams>,
) -> impl IntoResponse {
    callback_handler(state, auth_session, session, query).await
}

pub async fn callback_handler<I: IdentityProvider + 'static>(
    State(provider): State<Arc<I>>,
    mut auth_session: AuthSession,
    session: Session,
    Query(params): Query<CallbackParams>,
) -> impl IntoResponse {
    // 1. Verify CSRF state
    let expected_state: Option<String> = session.get("csrf_token").await.ok().flatten();
    if expected_state.as_deref() != Some(params.state.as_str()) {
        return Redirect::to("/?error=csrf_mismatch");
    }

    // // 2. Retrieve stored nonce
    let Some(nonce): Option<String> = session.get("nonce").await.ok().flatten() else {
        return Redirect::to("/?error=missing_nonce");
    };

    // 3. Exchange code, verify ID token + nonce
    let claims = match provider.exchange_code(params.code, nonce).await {
        Ok(claims) => claims,
        Err(e) => {
            eprintln!("token exchange failed: {e:?}");
            return Redirect::to("/?error=exchange_failed");
        }
    };

    // Clean up one-time-use session values
    session.remove::<String>("csrf_token").await.ok();
    session.remove::<String>("nonce").await.ok();

    // 4. Authenticate + establish session
    let creds = OidcCredentials {
        sub: claims.sub,
        email: claims.email,
        name: claims.name,
    };

    match auth_session.authenticate(creds).await {
        Ok(Some(user)) => {
            if auth_session.login(&user).await.is_err() {
                return Redirect::to("/?error=login_failed");
            }
            Redirect::to("/protected")
        }
        _ => Redirect::to("/?error=no_user"),
    }
}

pub async fn logout_handler(mut auth_session: AuthSession) -> impl IntoResponse {
    auth_session.logout().await.ok();
    Redirect::to("/")
}

pub async fn protected_handler(auth_session: AuthSession) -> impl IntoResponse {
    match auth_session.user {
        Some(user) => format!("Hello, {}! You are logged in.", user.email),
        None => "Not logged in (you shouldn't see this).".to_string(),
    }
}

pub async fn index_handler() -> impl IntoResponse {
    "Home page. Try /login or /protected."
}

pub async fn my_assets_handler() -> Json<Value> {
    let page = serde_json::from_str(include_str!("../data/my-assets.json"))
        .expect("my-assets.json must contain valid JSON");
    Json(page)
}

pub async fn shubham_handler() -> impl IntoResponse {
    "Shubham page. Try /login or /protected."
}
