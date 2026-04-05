mod auth;
mod root;

use axum::Router;

use crate::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new().merge(auth::router()).merge(root::router())
}
