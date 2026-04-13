mod auth;
mod bookmark;
mod root;

use axum::Router;

use crate::AppState;

pub(crate) fn router(base_path: &str) -> Router<AppState> {
    let inner = Router::new()
        .merge(auth::router())
        .merge(bookmark::router())
        .merge(root::router());
    if base_path.is_empty() {
        inner
    } else {
        Router::new().nest(base_path, inner)
    }
}
