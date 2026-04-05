mod callback;
mod login;

use axum::Router;

use crate::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .merge(callback::router())
        .merge(login::router())
}
