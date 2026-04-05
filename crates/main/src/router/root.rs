use axum::Router;
use axum::routing::get;

use crate::AppState;
use crate::extractor::{self};

pub(crate) fn router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(
            |extractor::RequireAuth(claims): extractor::RequireAuth| async move {
                format!("OK: {}", claims.sub)
            },
        ),
    )
}
