use axum::Router;
use axum::extract::State;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::AppState;
use crate::extractor::RequireAuth;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/", get(handler))
}

async fn handler(State(state): State<AppState>, auth: Option<RequireAuth>) -> impl IntoResponse {
    match auth {
        Some(RequireAuth(claims)) => Html(format!("OK: {}", claims.sub)).into_response(),
        None => {
            let base = &state.base_path;
            Html(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>shiori</title></head>
<body>
<h1>shiori</h1>
<p><a href="{base}/auth/signup">Sign Up</a></p>
<p><a href="{base}/auth/signin">Sign In</a></p>
</body>
</html>"#
            ))
            .into_response()
        }
    }
}
