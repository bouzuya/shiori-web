use crate::AppState;

const SHIORI_SVG: &str = include_str!("../../assets/shiori.svg");

pub(crate) fn router() -> ::axum::Router<AppState> {
    ::axum::Router::new().route("/shiori.svg", ::axum::routing::get(handler))
}

async fn handler() -> impl ::axum::response::IntoResponse {
    (
        [(
            ::axum::http::header::CONTENT_TYPE,
            "image/svg+xml; charset=utf-8",
        )],
        SHIORI_SVG,
    )
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_shiori_svg_returns_ok() -> ::anyhow::Result<()> {
        let response = send_request(
            test_app("shiori_svg_user")?,
            ::axum::http::Request::builder()
                .uri("/shiori.svg")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_shiori_svg_returns_svg_content_type() -> ::anyhow::Result<()> {
        let response = send_request(
            test_app("shiori_svg_ct_user")?,
            ::axum::http::Request::builder()
                .uri("/shiori.svg")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let content_type = response
            .headers()
            .get(::axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("image/svg+xml"),
            "Expected image/svg+xml content type, got: {content_type}"
        );
        Ok(())
    }
}
