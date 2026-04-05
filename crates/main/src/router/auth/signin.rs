use axum::Router;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Cookie;

use crate::AppState;
use crate::cookie::FLOW_COOKIE;
use crate::cookie::NONCE_COOKIE;
use crate::cookie::STATE_COOKIE;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/signin", get(handler))
}

async fn handler(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    tracing::info!("auth signin: generating authentication request");
    let auth_request = state.oidc_client.build_authentication_request();
    tracing::debug!(url = %auth_request.url, "auth signin: redirecting to OIDC provider");
    let jar = jar
        .add(Cookie::new(FLOW_COOKIE, "signin".to_string()))
        .add(Cookie::new(NONCE_COOKIE, auth_request.nonce))
        .add(Cookie::new(STATE_COOKIE, auth_request.state));
    (jar, Redirect::temporary(&auth_request.url))
}
