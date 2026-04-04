use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use axum::{Router, routing::get};
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Cookie;

use crate::extractor::AppState;

const NONCE_COOKIE: &str = "oidc_nonce";
const STATE_COOKIE: &str = "oidc_state";

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/login", get(handler))
}

async fn handler(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    let auth_request = state.oidc_client.build_authentication_request();
    let jar = jar
        .add(Cookie::new(NONCE_COOKIE, auth_request.nonce))
        .add(Cookie::new(STATE_COOKIE, auth_request.state));
    (jar, Redirect::temporary(&auth_request.url))
}
