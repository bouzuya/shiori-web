use axum::Router;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::AppState;

const INDEX_CSS: &str = include_str!("../../assets/index.css");

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/index.css", get(handler))
}

async fn handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        INDEX_CSS,
    )
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;

    #[tokio::test]
    #[serial_test::serial]
    async fn get_index_css_returns_ok() -> anyhow::Result<()> {
        let response = send_request(
            test_app("index_css_user")?,
            axum::http::Request::builder()
                .uri("/index.css")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn get_index_css_returns_text_css_content_type() -> anyhow::Result<()> {
        let response = send_request(
            test_app("index_css_ct_user")?,
            axum::http::Request::builder()
                .uri("/index.css")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("text/css"),
            "Expected text/css content type, got: {content_type}"
        );
        Ok(())
    }
}
