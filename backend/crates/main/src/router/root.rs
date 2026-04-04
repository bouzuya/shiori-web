use axum::{Router, routing::get};

use crate::extractor::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/", get(|| async { "OK" }))
}
