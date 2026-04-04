use axum::{Router, routing::get};

use crate::extractor::{self, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(|extractor::RequireAuth(claims): extractor::RequireAuth| async move {
            format!("OK: {}", claims.sub)
        }),
    )
}
