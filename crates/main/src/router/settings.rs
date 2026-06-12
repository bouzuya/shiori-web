use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;

use crate::AppState;
use crate::extractor::CurrentUserId;

pub(crate) fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/settings", axum::routing::get(get_settings))
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate<'a> {
    base: &'a str,
    color_scheme: &'a str,
}

async fn get_settings(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let color_scheme = super::resolve_color_scheme(&state, user_id).await;
    let template = SettingsTemplate {
        base: &state.base_path,
        color_scheme: &color_scheme,
    };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("template render failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::header;

    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;
    use crate::test_helpers::unique_user_id;

    async fn session_cookie(app: axum::Router, sub: &str) -> anyhow::Result<String> {
        let signup = send_request(
            app.clone(),
            Request::builder().uri("/auth/signup").body(Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup);
        let callback = send_request(
            app.clone(),
            Request::builder()
                .uri("/auth/callback?code=test&state=test_state")
                .header(header::COOKIE, &cookie_header)
                .body(Body::empty())?,
        )
        .await?;
        let session = callback
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .find_map(|v| {
                let s = v.to_str().ok()?;
                if !s.contains("session") {
                    return None;
                }
                s.split(';').next().map(|p| p.to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("session cookie not found for {sub}"))?;
        Ok(session)
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_settings_returns_html() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .uri("/settings")
                .header(header::COOKIE, session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(body.contains("Settings"));
        assert!(body.contains("data-color-scheme="));
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_settings_requires_auth() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let response = send_request(
            app,
            Request::builder().uri("/settings").body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }
}
