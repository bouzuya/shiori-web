use axum::Router;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::CookieJar;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/signout", get(handler))
}

async fn handler(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    tracing::info!("auth signout: removing session cookie");
    let jar = jar.with_signout_cookies();
    let redirect_target = if state.base_path.is_empty() {
        "/".to_string()
    } else {
        state.base_path.clone()
    };
    (jar, Redirect::temporary(&redirect_target))
}
