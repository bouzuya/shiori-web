mod env;
mod extractor;
mod router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let env = env::Env::from_env()?;
    let state = extractor::AppState::from_env(&env).await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on 0.0.0.0:3000");
    axum::serve(listener, router::router().with_state(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::extractor::AppState;
    use crate::extractor::{self};

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
    async fn get_auth_login_redirects_to_oidc_provider() -> anyhow::Result<()> {
        let response = send_request(
            test_app(),
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri("/auth/login")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        let location = response.headers().get("location").unwrap().to_str()?;
        assert!(
            location.starts_with("https://provider.example.com/authorize"),
            "Expected redirect to OIDC provider, got: {location}"
        );
        let set_cookies: Vec<_> = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        assert!(
            set_cookies.iter().any(|c| c.contains("oidc_state")),
            "Expected oidc_state cookie to be set"
        );
        assert!(
            set_cookies.iter().any(|c| c.contains("oidc_nonce")),
            "Expected oidc_nonce cookie to be set"
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_auth_callback_sets_session_and_redirects() -> anyhow::Result<()> {
        // Step 1: Login to get CSRF and nonce cookies
        let state = AppState::new(Arc::new(MockOidcClient));
        let login_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/login")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookies: Vec<_> = login_response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        let cookie_header = cookies.join("; ");

        // Step 2: Call callback with code, state, and cookies from login
        let response = send_request(
            crate::router::router().with_state(state),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header("cookie", &cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        let location = response.headers().get("location").unwrap().to_str()?;
        assert_eq!(location, "/");
        let set_cookies: Vec<_> = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        assert!(
            set_cookies.iter().any(|c| c.contains("session")),
            "Expected session cookie to be set"
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_protected_without_session_returns_unauthorized() -> anyhow::Result<()> {
        let response = send_request(
            test_app(),
            axum::http::Request::builder()
                .uri("/")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn get_protected_with_session_returns_ok() -> anyhow::Result<()> {
        // Full flow: login → callback → access protected route
        let state = AppState::new(Arc::new(MockOidcClient));

        // Step 1: Login
        let login_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/login")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let login_cookies: Vec<_> = login_response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        let login_cookie_header = login_cookies.join("; ");

        // Step 2: Callback
        let callback_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header("cookie", &login_cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let callback_cookies: Vec<_> = callback_response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        let session_cookie_header = callback_cookies.join("; ");

        // Step 3: Access protected route with session cookie
        let response = send_request(
            crate::router::router().with_state(state),
            axum::http::Request::builder()
                .uri("/")
                .header("cookie", &session_cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.starts_with("OK: "),
            "Expected body to start with 'OK: ', got: {body}"
        );
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
