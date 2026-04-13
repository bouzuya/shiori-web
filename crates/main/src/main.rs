mod cookie_jar;
mod env;
mod extractor;
mod model;
mod router;
mod state;
#[cfg(test)]
mod test_helpers;

pub(crate) use self::cookie_jar::CookieJar;
pub(crate) use self::state::AppState;

fn generate_secret() -> String {
    use axum_extra::extract::cookie::Key;
    let key = Key::generate();
    key.master().iter().map(|b| format!("{b:02x}")).collect()
}

async fn run_server() -> anyhow::Result<()> {
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
    axum::serve(listener, router::router(&env.base_path).with_state(state)).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("generate-secret") {
        println!("{}", generate_secret());
        return Ok(());
    }
    run_server().await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[test]
    fn generate_secret_returns_string_usable_as_cookie_signing_secret() {
        use axum_extra::extract::cookie::Key;
        let secret = super::generate_secret();
        // cookie_signing_secret は Key::from() に渡すため、UTF-8 バイト列が 64 バイト以上必要
        assert!(
            secret.len() >= 64,
            "generated secret must be at least 64 bytes, got {}",
            secret.len()
        );
        // 実際に Key::from() で変換できることを確認
        let _ = Key::from(secret.as_bytes());
    }

    use crate::AppState;
    use crate::model::FirestoreBookmarkRepository;
    use crate::model::FirestoreUserRepository;
    use crate::model::User;
    use crate::model::UserRepository;
    use crate::test_helpers::MockOidcClient;

    struct MockUserRepository {
        users: std::sync::Mutex<Vec<crate::model::User>>,
    }

    impl MockUserRepository {
        fn new() -> Self {
            Self {
                users: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl UserRepository for MockUserRepository {
        async fn find(
            &self,
            id: &crate::model::UserId,
        ) -> anyhow::Result<Option<crate::model::User>> {
            let users = self.users.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(users.iter().find(|u| u.id() == *id).cloned())
        }

        async fn find_by_google_user_id(
            &self,
            id: &crate::model::GoogleUserId,
        ) -> anyhow::Result<Option<crate::model::User>> {
            let users = self.users.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(users.iter().find(|u| u.google_user_id() == id).cloned())
        }

        async fn store(&self, user: crate::model::User) -> anyhow::Result<()> {
            let mut users = self.users.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            if let Some(pos) = users.iter().position(|u| u.id() == user.id()) {
                users[pos] = user;
            } else {
                users.push(user);
            }
            Ok(())
        }
    }

    const TEST_COOKIE_SIGNING_SECRET: &str =
        "test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding";

    fn unique_user_id() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("u{nanos}")
    }

    fn firestore() -> anyhow::Result<bouzuya_firestore_client::Firestore> {
        Ok(bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?)
    }

    fn firestore_user_repo() -> anyhow::Result<Arc<dyn UserRepository>> {
        Ok(Arc::new(FirestoreUserRepository::new(firestore()?)))
    }

    fn firestore_bookmark_repo() -> anyhow::Result<Arc<dyn crate::model::BookmarkRepository>> {
        Ok(Arc::new(FirestoreBookmarkRepository::new(firestore()?)))
    }

    fn test_app(sub: impl Into<String>) -> anyhow::Result<axum::Router> {
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(sub)),
            firestore_user_repo()?,
        );
        Ok(crate::router::router("").with_state(state))
    }

    fn test_app_with_mock_repo(sub: impl Into<String>) -> axum::Router {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )
        .expect("firestore");
        let bookmark_repository = Arc::new(FirestoreBookmarkRepository::new(firestore))
            as Arc<dyn crate::model::BookmarkRepository>;
        let state = AppState::new(
            "".to_string(),
            bookmark_repository,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(sub)),
            Arc::new(MockUserRepository::new()),
        );
        crate::router::router("").with_state(state)
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn get_auth_signup_redirects_to_oidc_provider() -> anyhow::Result<()> {
        let response = send_request(
            test_app("test_signup_redirect_user")?,
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
    #[serial_test::serial]
    async fn get_auth_signin_redirects_to_oidc_provider() -> anyhow::Result<()> {
        let response = send_request(
            test_app("test_signin_redirect_user")?,
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
    #[serial_test::serial]
    async fn signup_callback_creates_user_and_sets_session() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );

        // Step 1: Signup to get CSRF and nonce cookies
        let signup_response = send_request(
            crate::router::router("").with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup_response);

        // Step 2: Call callback with code, state, and cookies
        let response = send_request(
            crate::router::router("").with_state(state),
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
    #[serial_test::serial]
    async fn signin_callback_with_existing_user_sets_session() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let user_repo = firestore_user_repo()?;
        user_repo
            .store(User::create(sub.parse::<crate::model::GoogleUserId>()?))
            .await?;
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            user_repo,
        );

        // Step 1: Signin
        let signin_response = send_request(
            crate::router::router("").with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signin")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signin_response);

        // Step 2: Callback
        let response = send_request(
            crate::router::router("").with_state(state),
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
    #[serial_test::serial]
    async fn signin_callback_with_unknown_user_returns_error() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );

        // Step 1: Signin (no user in DB)
        let signin_response = send_request(
            crate::router::router("").with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signin")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signin_response);

        // Step 2: Callback — should fail because user doesn't exist
        let response = send_request(
            crate::router::router("").with_state(state),
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
    #[serial_test::serial]
    async fn get_root_without_session_returns_landing_page() -> anyhow::Result<()> {
        let response = send_request(
            test_app("test_root_no_session_user")?,
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
    #[serial_test::serial]
    async fn get_root_with_session_returns_ok() -> anyhow::Result<()> {
        // Full flow: signup → callback → access root
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );

        // Step 1: Signup
        let signup_response = send_request(
            crate::router::router("").with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let signup_cookie_header = extract_cookies(&signup_response);

        // Step 2: Callback
        let callback_response = send_request(
            crate::router::router("").with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header(axum::http::header::COOKIE, &signup_cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let session_cookie_header = extract_cookies(&callback_response);

        // Step 3: Access root with session cookie
        let response = send_request(
            crate::router::router("").with_state(state),
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

    // --- BASE_PATH tests ---

    #[tokio::test]
    #[serial_test::serial]
    async fn with_base_path_routes_are_under_base_path() -> anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new("base_path_route_user")),
            firestore_user_repo()?,
        );

        // Route exists under base path
        let response = send_request(
            crate::router::router(base_path).with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/app/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT,
            "Expected route under base path to exist"
        );

        // Route does NOT exist without base path
        let response = send_request(
            crate::router::router(base_path).with_state(state),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::NOT_FOUND,
            "Expected route without base path to return 404"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn with_base_path_callback_redirects_to_base_path() -> anyhow::Result<()> {
        let base_path = "/app";
        let sub = unique_user_id();
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );

        // Step 1: Signup
        let signup_response = send_request(
            crate::router::router(base_path).with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/app/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup_response);

        // Step 2: Callback — redirect target should be base_path
        let response = send_request(
            crate::router::router(base_path).with_state(state),
            axum::http::Request::builder()
                .uri("/app/auth/callback?code=test_code&state=test_state")
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
        assert_eq!(location, "/app");
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn with_base_path_session_cookie_has_base_path() -> anyhow::Result<()> {
        let base_path = "/app";
        let sub = unique_user_id();
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );

        // Step 1: Signup
        let signup_response = send_request(
            crate::router::router(base_path).with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/app/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup_response);

        // Step 2: Callback — session cookie Path should be base_path
        let response = send_request(
            crate::router::router(base_path).with_state(state),
            axum::http::Request::builder()
                .uri("/app/auth/callback?code=test_code&state=test_state")
                .header(axum::http::header::COOKIE, &cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let set_cookies: Vec<_> = response
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        assert!(
            set_cookies
                .iter()
                .any(|c| c.contains("session") && c.contains("Path=/app")),
            "Expected session cookie with Path=/app, got: {set_cookies:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_auth_signout_redirects_to_root() -> anyhow::Result<()> {
        let response = send_request(
            test_app_with_mock_repo("test_signout_redirect_user"),
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri("/auth/signout")
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
        Ok(())
    }

    #[tokio::test]
    async fn get_auth_signout_clears_session_cookie() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            Arc::new(MockUserRepository::new()),
        );

        // Step 1: Signup
        let signup_response = send_request(
            crate::router::router("").with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let signup_cookie_header = extract_cookies(&signup_response);

        // Step 2: Callback
        let callback_response = send_request(
            crate::router::router("").with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header(axum::http::header::COOKIE, &signup_cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let session_cookie_header = extract_cookies(&callback_response);

        // Step 3: Signout
        let response = send_request(
            crate::router::router("").with_state(state),
            axum::http::Request::builder()
                .uri("/auth/signout")
                .header(axum::http::header::COOKIE, &session_cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT
        );
        let set_cookies: Vec<_> = response
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        assert!(
            set_cookies
                .iter()
                .any(|c| c.contains("session") && c.contains("Max-Age=0")),
            "Expected session cookie to be cleared, got: {set_cookies:?}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn with_base_path_signout_redirects_to_base_path() -> anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new("signout_base_path_user")),
            Arc::new(MockUserRepository::new()),
        );
        let response = send_request(
            crate::router::router(base_path).with_state(state),
            axum::http::Request::builder()
                .uri("/app/auth/signout")
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
        assert_eq!(location, "/app");
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn with_base_path_root_contains_base_path_links() -> anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new("base_path_links_user")),
            firestore_user_repo()?,
        );
        let response = send_request(
            crate::router::router(base_path).with_state(state),
            axum::http::Request::builder()
                .uri("/app")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("/app/auth/signup"),
            "Expected landing page to contain /app/auth/signup link, got: {body}"
        );
        assert!(
            body.contains("/app/auth/signin"),
            "Expected landing page to contain /app/auth/signin link, got: {body}"
        );
        Ok(())
    }

    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::send_request;
}
