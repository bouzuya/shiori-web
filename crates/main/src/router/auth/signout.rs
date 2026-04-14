use axum::Router;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::CookieJar;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/signout", get(handler))
}

async fn handler(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    tracing::info!("auth signout: removing session cookie");
    let jar = jar.with_signout_cookies();
    let redirect_target = if state.base_path.is_empty() {
        "/".to_string()
    } else {
        state.base_path.clone()
    };
    (jar, Redirect::temporary(&redirect_target))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::AppState;
    use crate::test_helpers::MockOidcClient;
    use crate::test_helpers::MockUserRepository;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app_with_mock_repo;
    use crate::test_helpers::unique_user_id;

    #[tokio::test]
    async fn get_auth_signout_redirects_to_root() -> anyhow::Result<()> {
        let response = send_request(
            test_app_with_mock_repo("test_signout_redirect_user")?,
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
}
