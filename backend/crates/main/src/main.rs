mod env;
mod extractor;
mod router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env = env::Env::from_env()?;
    let state = extractor::AppState::from_env(&env).await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, router::router().with_state(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::extractor::{self, AppState};

    struct MockOidcClient;

    #[async_trait::async_trait]
    impl extractor::OidcClient for MockOidcClient {
        fn build_authentication_request(&self) -> extractor::AuthenticationRequest {
            extractor::AuthenticationRequest {
                nonce: "test_nonce".to_string(),
                state: "test_state".to_string(),
                url: "https://provider.example.com/authorize?client_id=test".to_string(),
            }
        }

        async fn exchange_code(
            &self,
            _code: &str,
            _nonce: &str,
        ) -> anyhow::Result<extractor::OidcClaims> {
            Ok(extractor::OidcClaims {
                sub: "user123".to_string(),
            })
        }
    }

    fn test_app() -> axum::Router {
        let state = AppState::new(Arc::new(MockOidcClient));
        crate::router::router().with_state(state)
    }

    #[tokio::test]
    async fn get_root_returns_ok() -> anyhow::Result<()> {
        let response = send_request(
            test_app(),
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
        router: axum::Router,
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
