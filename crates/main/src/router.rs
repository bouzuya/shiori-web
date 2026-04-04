mod auth;
mod root;

use axum::Router;

use crate::extractor::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new().merge(auth::router()).merge(root::router())
}
