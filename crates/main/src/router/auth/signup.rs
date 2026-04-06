use axum::Router;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::CookieJar;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/signup", get(handler))
}

async fn handler(State(state): State<AppState>, cookie_jar: CookieJar) -> impl IntoResponse {
    tracing::info!("auth signup: generating authentication request");
    let auth_request = state.oidc_client.build_authentication_request();
    tracing::debug!(url = %auth_request.url, "auth signup: redirecting to OIDC provider");
    (
        cookie_jar.with_signup_cookies(&auth_request),
        Redirect::temporary(&auth_request.url),
    )
}
