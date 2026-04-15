use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::CookieJar;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/callback", get(handler))
}

#[derive(serde::Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

async fn handler(
    State(app_state): State<AppState>,
    jar: CookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("auth callback: received callback request");

    let csrf_state = jar.get_state().ok_or_else(|| {
        tracing::warn!("auth callback: oidc_state cookie not found, returning 400");
        StatusCode::BAD_REQUEST
    })?;
    if params.state != csrf_state {
        tracing::warn!("auth callback: CSRF state mismatch, returning 400");
        return Err(StatusCode::BAD_REQUEST);
    }

    let nonce = jar.get_nonce().ok_or_else(|| {
        tracing::warn!("auth callback: oidc_nonce cookie not found, returning 400");
        StatusCode::BAD_REQUEST
    })?;

    let flow = jar.get_flow().ok_or_else(|| {
        tracing::warn!("auth callback: auth_flow cookie not found, returning 400");
        StatusCode::BAD_REQUEST
    })?;

    let oidc_claims = app_state
        .oidc_client
        .exchange_code(&params.code, &nonce)
        .await
        .map_err(|e| {
            tracing::error!("auth callback: failed to exchange code: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let google_user_id = oidc_claims
        .sub
        .parse::<crate::model::GoogleUserId>()
        .map_err(|e| {
            tracing::error!("auth callback: invalid google user id: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let user = app_state
        .user_repository
        .find_by_google_user_id(&google_user_id)
        .await
        .map_err(|e| {
            tracing::error!("auth callback: failed to find user: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let user_id = match (flow.as_str(), user) {
        ("signin", None) => {
            tracing::warn!(
                sub = %oidc_claims.sub,
                "auth callback: user not found for signin, returning 403"
            );
            return Err(StatusCode::FORBIDDEN);
        }
        ("signin", Some(user)) => user.id(),
        ("signup", None) => {
            let new_user = crate::model::User::create(google_user_id);
            let user_id = new_user.id();
            app_state
                .user_repository
                .store(new_user)
                .await
                .map_err(|e| {
                    tracing::error!("auth callback: failed to store user: {e:?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            user_id
        }
        ("signup", Some(_)) => {
            tracing::warn!(
                sub = %oidc_claims.sub,
                "auth callback: user already exists for signup, returning 403"
            );
            return Err(StatusCode::FORBIDDEN);
        }
        _ => {
            tracing::warn!(flow, "auth callback: unknown auth flow, returning 400");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    tracing::info!(sub = %oidc_claims.sub, "auth callback: authentication successful, setting session cookie");
    let jar = jar.with_session_cookies(user_id.to_string());

    let redirect_target = if app_state.base_path.is_empty() {
        "/".to_string()
    } else {
        app_state.base_path.clone()
    };
    Ok((jar, Redirect::temporary(&redirect_target)))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::AppState;
    use crate::model::User;
    use crate::test_helpers::MockOidcClient;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_repo;
    use crate::test_helpers::send_request;
    use crate::test_helpers::unique_user_id;

    #[tokio::test]
    #[serial_test::serial]
    async fn signup_callback_creates_user_and_sets_session() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
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
            firestore_bookmark_reader()?,
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
            firestore_bookmark_reader()?,
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
    async fn with_base_path_callback_redirects_to_base_path() -> anyhow::Result<()> {
        let base_path = "/app";
        let sub = unique_user_id();
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
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
            firestore_bookmark_reader()?,
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
}
