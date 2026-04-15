use axum::Router;
use axum::extract::State;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::AppState;
use crate::extractor::CurrentUserId;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/", get(handler))
}

async fn handler(State(state): State<AppState>, auth: Option<CurrentUserId>) -> impl IntoResponse {
    match auth {
        Some(CurrentUserId(user_id)) => Html(format!("OK: {}", user_id)).into_response(),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::AppState;
    use crate::test_helpers::MockOidcClient;
    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_repo;
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;
    use crate::test_helpers::unique_user_id;

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
            firestore_bookmark_reader()?,
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

    #[tokio::test]
    #[serial_test::serial]
    async fn with_base_path_root_contains_base_path_links() -> anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
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
}
