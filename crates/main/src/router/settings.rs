use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Redirect;

use crate::AppState;
use crate::extractor::CurrentUserId;

pub(crate) fn router() -> axum::Router<AppState> {
    axum::Router::new().route(
        "/settings",
        axum::routing::get(get_settings).post(post_settings_dispatch),
    )
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate<'a> {
    base: &'a str,
    color_scheme: &'a str,
    utc_offset: &'a str,
}

async fn get_settings(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let color_scheme = super::resolve_color_scheme(&state, user_id).await;
    let utc_offset = super::resolve_utc_offset(&state, user_id).await.to_string();
    let template = SettingsTemplate {
        base: &state.base_path,
        color_scheme: &color_scheme,
        utc_offset: &utc_offset,
    };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("template render failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(serde::Deserialize)]
struct MethodOverrideQuery {
    #[serde(rename = "_method")]
    method: Option<String>,
}

#[derive(serde::Deserialize)]
struct PutSettingsRequest {
    color_scheme: String,
    utc_offset: String,
}

async fn post_settings_dispatch(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<MethodOverrideQuery>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    match query.method.as_deref() {
        Some("PUT") => {
            let form = match serde_urlencoded::from_bytes::<PutSettingsRequest>(&body) {
                Ok(f) => f,
                Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
            };
            put_settings_impl(user_id, state, form).await
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

async fn put_settings_impl(
    user_id: kernel::UserId,
    state: AppState,
    body: PutSettingsRequest,
) -> axum::response::Response {
    let color_scheme = match body.color_scheme.parse::<kernel::ColorScheme>() {
        Ok(cs) => cs,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    let utc_offset = match body.utc_offset.parse::<kernel::UtcOffset>() {
        Ok(o) => o,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    let settings = kernel::UserSettings::new(color_scheme, user_id, utc_offset);
    if let Err(e) = state.user_settings_repository.store(settings).await {
        tracing::error!("failed to store user settings: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let redirect_url = if state.base_path.is_empty() {
        "/settings".to_string()
    } else {
        format!("{}/settings", state.base_path)
    };
    Redirect::to(&redirect_url).into_response()
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
        assert!(
            body.contains("_method=PUT"),
            "Expected form with _method=PUT, got: {body}"
        );
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

    #[tokio::test]
    #[serial_test::serial]
    async fn test_put_settings_saves_and_redirects() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(Body::from("color_scheme=dark&utc_offset=%2B09%3A00"))?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        let get_response = send_request(
            app,
            Request::builder()
                .uri("/settings")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        let body = get_response.into_body_string().await?;
        assert!(
            body.contains(r#"data-color-scheme="dark""#),
            "Expected dark color scheme, got: {body}"
        );
        assert!(
            body.contains(r#"<option value="+09:00" selected>"#),
            "Expected +09:00 option to be selected, got: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_put_settings_rejects_invalid_color_scheme() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(Body::from("color_scheme=invalid&utc_offset=%2B09%3A00"))?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_put_settings_rejects_invalid_utc_offset() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(Body::from("color_scheme=dark&utc_offset=invalid"))?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_put_settings_requires_auth() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("color_scheme=dark"))?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_post_without_method_override_returns_405() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/settings")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(Body::from("color_scheme=dark"))?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::METHOD_NOT_ALLOWED
        );
        Ok(())
    }
}
