use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Cookie;

use crate::AppState;
use crate::cookie::FLOW_COOKIE;
use crate::cookie::NONCE_COOKIE;
use crate::cookie::SESSION_COOKIE;
use crate::cookie::STATE_COOKIE;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/callback", get(handler))
}

#[derive(serde::Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

async fn handler(
    State(app_state): State<AppState>,
    jar: SignedCookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("auth callback: received callback request");

    let csrf_state = jar
        .get(STATE_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            tracing::warn!("auth callback: oidc_state cookie not found, returning 400");
            StatusCode::BAD_REQUEST
        })?;
    if params.state != csrf_state {
        tracing::warn!("auth callback: CSRF state mismatch, returning 400");
        return Err(StatusCode::BAD_REQUEST);
    }

    let nonce = jar
        .get(NONCE_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            tracing::warn!("auth callback: oidc_nonce cookie not found, returning 400");
            StatusCode::BAD_REQUEST
        })?;

    let flow = jar
        .get(FLOW_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            tracing::warn!("auth callback: auth_flow cookie not found, returning 400");
            StatusCode::BAD_REQUEST
        })?;

    let oidc_claims = app_state
        .oidc_client
        .exchange_code(&params.code, &nonce)
        .await
        .map_err(|e| {
            tracing::error!("auth callback: failed to exchange code: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let user = app_state
        .user_repository
        .find(&oidc_claims.sub)
        .await
        .map_err(|e| {
            tracing::error!("auth callback: failed to find user: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    match (flow.as_str(), user) {
        ("signin", None) => {
            tracing::warn!(
                sub = %oidc_claims.sub,
                "auth callback: user not found for signin, returning 403"
            );
            return Err(StatusCode::FORBIDDEN);
        }
        ("signin", Some(_)) => {
            // signin successful, do nothing here and let the session be created below
        }
        ("signup", None) => {
            app_state
                .user_repository
                .store(crate::user::User::create(&oidc_claims.sub))
                .await
                .map_err(|e| {
                    tracing::error!("auth callback: failed to store user: {e:?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
        ("signup", Some(_)) => {
            tracing::warn!(
                sub = %oidc_claims.sub,
                "auth callback: user already exists for signup, returning 403"
            );
            return Err(StatusCode::FORBIDDEN);
        }
        _ => {
            tracing::warn!(flow, "auth callback: unknown auth flow, returning 400");
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let session_value = serde_json::to_string(&oidc_claims).map_err(|e| {
        tracing::error!("auth callback: failed to serialize session claims: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(sub = %oidc_claims.sub, "auth callback: authentication successful, setting session cookie");

    let jar = jar
        .remove(Cookie::from(FLOW_COOKIE))
        .remove(Cookie::from(STATE_COOKIE))
        .remove(Cookie::from(NONCE_COOKIE))
        .add(Cookie::build((SESSION_COOKIE, session_value)).path("/"));

    Ok((jar, Redirect::temporary("/")))
}
