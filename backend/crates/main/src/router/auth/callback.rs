use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Cookie;

use crate::extractor::AppState;

const SESSION_COOKIE: &str = "session";
const NONCE_COOKIE: &str = "oidc_nonce";
const STATE_COOKIE: &str = "oidc_state";

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
    let csrf_state = jar
        .get(STATE_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or(StatusCode::BAD_REQUEST)?;
    if params.state != csrf_state {
        return Err(StatusCode::BAD_REQUEST);
    }

    let nonce = jar
        .get(NONCE_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let claims = app_state
        .oidc_client
        .exchange_code(&params.code, &nonce)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_value =
        serde_json::to_string(&claims).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let jar = jar
        .remove(Cookie::from(STATE_COOKIE))
        .remove(Cookie::from(NONCE_COOKIE))
        .add(Cookie::new(SESSION_COOKIE, session_value));

    Ok((jar, Redirect::temporary("/")))
}
