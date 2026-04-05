mod callback;
mod signin;
mod signup;

use axum::Router;

use crate::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .merge(callback::router())
        .merge(signin::router())
        .merge(signup::router())
}
