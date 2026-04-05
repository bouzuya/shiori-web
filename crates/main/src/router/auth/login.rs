use axum::Router;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Cookie;

use crate::AppState;

const NONCE_COOKIE: &str = "oidc_nonce";
const STATE_COOKIE: &str = "oidc_state";

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/login", get(handler))
}

async fn handler(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    tracing::info!("auth login: generating authentication request");
    let auth_request = state.oidc_client.build_authentication_request();
    tracing::debug!(url = %auth_request.url, "auth login: redirecting to OIDC provider");
    let jar = jar
        .add(Cookie::new(NONCE_COOKIE, auth_request.nonce))
        .add(Cookie::new(STATE_COOKIE, auth_request.state));
    (jar, Redirect::temporary(&auth_request.url))
}
