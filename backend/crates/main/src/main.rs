use axum::{Router, routing::get};

fn app() -> Router {
    Router::new().route("/", get(|| async { "OK" }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_root_returns_ok() -> anyhow::Result<()> {
        let response = send_request(
            app(),
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri("/")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.into_body_string().await?, "OK");
        Ok(())
    }

    async fn send_request(
        router: axum::Router<()>,
        request: axum::http::Request<axum::body::Body>,
    ) -> anyhow::Result<axum::response::Response<axum::body::Body>> {
        let response = tower::ServiceExt::oneshot(router, request).await?;
        Ok(response)
    }

    trait ResponseExt {
        async fn into_body_string(self) -> anyhow::Result<String>;
    }

    impl ResponseExt for axum::response::Response<axum::body::Body> {
        async fn into_body_string(self) -> anyhow::Result<String> {
            let bytes = axum::body::to_bytes(self.into_body(), usize::MAX).await?;
            Ok(String::from_utf8(bytes.to_vec())?)
        }
    }
}
