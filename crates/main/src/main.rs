mod cookie_jar;
mod env;
mod extractor;
mod router;
mod state;
mod user;

pub(crate) use self::cookie_jar::CookieJar;
pub(crate) use self::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let env = env::Env::from_env()?;
    let state = AppState::from_env(&env).await?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on 0.0.0.0:3000");
    axum::serve(listener, router::router().with_state(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::AppState;
    use crate::extractor::{self};
    use crate::user::InMemoryUserRepository;
    use crate::user::User;
    use crate::user::UserRepository;

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
        let state = AppState::new(
            Arc::new(MockOidcClient),
            Arc::new(InMemoryUserRepository::new()),
        );
        crate::router::router().with_state(state)
    }

    #[tokio::test]
    async fn get_auth_signup_redirects_to_oidc_provider() -> anyhow::Result<()> {
        let response = send_request(
            test_app(),
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        let location = response
            .headers()
            .get(axum::http::header::LOCATION)
            .expect("Expected location header")
            .to_str()?;
        assert!(
            location.starts_with("https://provider.example.com/authorize"),
            "Expected redirect to OIDC provider, got: {location}"
        );
        let set_cookies: Vec<_> = response
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        assert!(
            set_cookies.iter().any(|c| c.contains("oidc_state")),
            "Expected oidc_state cookie to be set"
        );
        assert!(
            set_cookies.iter().any(|c| c.contains("oidc_nonce")),
            "Expected oidc_nonce cookie to be set"
        );
        assert!(
            set_cookies.iter().any(|c| c.contains("auth_flow")),
            "Expected auth_flow cookie to be set"
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_auth_signin_redirects_to_oidc_provider() -> anyhow::Result<()> {
        let response = send_request(
            test_app(),
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri("/auth/signin")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        let location = response
            .headers()
            .get(axum::http::header::LOCATION)
            .expect("Expected location header")
            .to_str()?;
        assert!(
            location.starts_with("https://provider.example.com/authorize"),
            "Expected redirect to OIDC provider, got: {location}"
        );
        let set_cookies: Vec<_> = response
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        assert!(
            set_cookies.iter().any(|c| c.contains("auth_flow")),
            "Expected auth_flow cookie to be set"
        );
        Ok(())
    }

    #[tokio::test]
    async fn signup_callback_creates_user_and_sets_session() -> anyhow::Result<()> {
        let state = AppState::new(
            Arc::new(MockOidcClient),
            Arc::new(InMemoryUserRepository::new()),
        );

        // Step 1: Signup to get CSRF and nonce cookies
        let signup_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup_response);

        // Step 2: Call callback with code, state, and cookies
        let response = send_request(
            crate::router::router().with_state(state),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header(axum::http::header::COOKIE, &cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        let location = response
            .headers()
            .get(axum::http::header::LOCATION)
            .expect("Expected location header")
            .to_str()?;
        assert_eq!(location, "/");
        let set_cookies: Vec<_> = response
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        assert!(
            set_cookies.iter().any(|c| c.contains("session")),
            "Expected session cookie to be set"
        );
        Ok(())
    }

    #[tokio::test]
    async fn signin_callback_with_existing_user_sets_session() -> anyhow::Result<()> {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        user_repo.store(User::create("user123")).await?;
        let state = AppState::new(Arc::new(MockOidcClient), user_repo);

        // Step 1: Signin
        let signin_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signin")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signin_response);

        // Step 2: Callback
        let response = send_request(
            crate::router::router().with_state(state),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header(axum::http::header::COOKIE, &cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        Ok(())
    }

    #[tokio::test]
    async fn signin_callback_with_unknown_user_returns_error() -> anyhow::Result<()> {
        let state = AppState::new(
            Arc::new(MockOidcClient),
            Arc::new(InMemoryUserRepository::new()),
        );

        // Step 1: Signin (no user in DB)
        let signin_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signin")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signin_response);

        // Step 2: Callback — should fail because user doesn't exist
        let response = send_request(
            crate::router::router().with_state(state),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header(axum::http::header::COOKIE, &cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        Ok(())
    }

    #[tokio::test]
    async fn get_root_without_session_returns_landing_page() -> anyhow::Result<()> {
        let response = send_request(
            test_app(),
            axum::http::Request::builder()
                .uri("/")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("/auth/signup"),
            "Expected landing page to contain signup link"
        );
        assert!(
            body.contains("/auth/signin"),
            "Expected landing page to contain signin link"
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_root_with_session_returns_ok() -> anyhow::Result<()> {
        // Full flow: signup → callback → access root
        let state = AppState::new(
            Arc::new(MockOidcClient),
            Arc::new(InMemoryUserRepository::new()),
        );

        // Step 1: Signup
        let signup_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let signup_cookie_header = extract_cookies(&signup_response);

        // Step 2: Callback
        let callback_response = send_request(
            crate::router::router().with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header(axum::http::header::COOKIE, &signup_cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let session_cookie_header = extract_cookies(&callback_response);

        // Step 3: Access root with session cookie
        let response = send_request(
            crate::router::router().with_state(state),
            axum::http::Request::builder()
                .uri("/")
                .header(axum::http::header::COOKIE, &session_cookie_header)
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

    fn extract_cookies(response: &axum::response::Response<axum::body::Body>) -> String {
        response
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect::<Vec<_>>()
            .join("; ")
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
