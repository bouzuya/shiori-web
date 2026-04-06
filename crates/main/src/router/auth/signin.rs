use axum::Router;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::CookieJar;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/signin", get(handler))
}

async fn handler(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    tracing::info!("auth signin: generating authentication request");
    let auth_request = state.oidc_client.build_authentication_request();
    tracing::debug!(url = %auth_request.url, "auth signin: redirecting to OIDC provider");
    let jar = jar.with_signin_cookies(&auth_request);
    (jar, Redirect::temporary(&auth_request.url))
}
