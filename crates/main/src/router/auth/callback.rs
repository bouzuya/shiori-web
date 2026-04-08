use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::CookieJar;

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
    jar: CookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("auth callback: received callback request");

    let csrf_state = jar.get_state().ok_or_else(|| {
        tracing::warn!("auth callback: oidc_state cookie not found, returning 400");
        StatusCode::BAD_REQUEST
    })?;
    if params.state != csrf_state {
        tracing::warn!("auth callback: CSRF state mismatch, returning 400");
        return Err(StatusCode::BAD_REQUEST);
    }

    let nonce = jar.get_nonce().ok_or_else(|| {
        tracing::warn!("auth callback: oidc_nonce cookie not found, returning 400");
        StatusCode::BAD_REQUEST
    })?;

    let flow = jar.get_flow().ok_or_else(|| {
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
                .store(crate::model::User::create(
                    oidc_claims
                        .sub
                        .parse::<crate::model::UserId>()
                        .map_err(|e| {
                            tracing::error!("auth callback: invalid user id: {e:?}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?,
                ))
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

    tracing::info!(sub = %oidc_claims.sub, "auth callback: authentication successful, setting session cookie");
    let jar = jar.with_session_cookies(oidc_claims);

    Ok((jar, Redirect::temporary("/")))
}
