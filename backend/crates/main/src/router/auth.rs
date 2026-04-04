mod login;

use axum::Router;

use crate::extractor::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new().merge(login::router())
}
