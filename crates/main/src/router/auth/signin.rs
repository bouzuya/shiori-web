use axum::Router;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;

use crate::AppState;
use crate::CookieJar;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/auth/signin", get(handler))
}

async fn handler(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    tracing::info!("auth signin: generating authentication request");
    let auth_request = state.oidc_client.build_authentication_request();
    tracing::debug!(url = %auth_request.url, "auth signin: redirecting to OIDC provider");
    let jar = jar.with_signin_cookies(&auth_request);
    (jar, Redirect::temporary(&auth_request.url))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;

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
}
