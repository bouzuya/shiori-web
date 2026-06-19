use crate::AppState;
use crate::extractor::CurrentUserId;
use kernel::ColorScheme;
use kernel::ShareUrl;
use kernel::UserId;
use kernel::UserSettings;
use kernel::UtcOffset;

pub(crate) fn router() -> ::axum::Router<AppState> {
    ::axum::Router::new().route(
        "/settings",
        ::axum::routing::get(get_settings).post(post_settings_dispatch),
    )
}

#[derive(::askama::Template)]
#[template(path = "settings.html")]
struct SettingsTemplate<'a> {
    base: &'a str,
    color_scheme: &'a str,
    share_url: &'a str,
    utc_offset: &'a str,
}

async fn get_settings(
    CurrentUserId(user_id): CurrentUserId,
    ::axum::extract::State(state): ::axum::extract::State<AppState>,
) -> impl ::axum::response::IntoResponse {
    let color_scheme = super::resolve_color_scheme(&state, user_id).await;
    let share_url = super::resolve_share_url(&state, user_id)
        .await
        .map(|s| s.to_string())
        .unwrap_or_default();
    let utc_offset = super::resolve_utc_offset(&state, user_id).await.to_string();
    let template = SettingsTemplate {
        base: &state.base_path,
        color_scheme: &color_scheme,
        share_url: &share_url,
        utc_offset: &utc_offset,
    };
    match ::askama::Template::render(&template) {
        Ok(html) => ::axum::response::IntoResponse::into_response(::axum::response::Html(html)),
        Err(e) => {
            ::tracing::error!("template render failed: {e}");
            ::axum::response::IntoResponse::into_response(
                ::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

#[derive(::serde::Deserialize)]
struct MethodOverrideQuery {
    #[serde(rename = "_method")]
    method: Option<String>,
}

#[derive(::serde::Deserialize)]
struct PutSettingsRequest {
    color_scheme: String,
    // 未送信・空文字は未設定 (None) として扱う。
    #[serde(default)]
    share_url: String,
    utc_offset: String,
}

async fn post_settings_dispatch(
    CurrentUserId(user_id): CurrentUserId,
    ::axum::extract::State(state): ::axum::extract::State<AppState>,
    ::axum::extract::Query(query): ::axum::extract::Query<MethodOverrideQuery>,
    body: ::axum::body::Bytes,
) -> impl ::axum::response::IntoResponse {
    match query.method.as_deref() {
        Some("PUT") => {
            let form = match ::serde_urlencoded::from_bytes::<PutSettingsRequest>(&body) {
                Ok(f) => f,
                Err(_) => {
                    return ::axum::response::IntoResponse::into_response(
                        ::axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                    );
                }
            };
            put_settings_impl(user_id, state, form).await
        }
        _ => ::axum::response::IntoResponse::into_response(
            ::axum::http::StatusCode::METHOD_NOT_ALLOWED,
        ),
    }
}

async fn put_settings_impl(
    user_id: UserId,
    state: AppState,
    body: PutSettingsRequest,
) -> ::axum::response::Response {
    let color_scheme = match body.color_scheme.parse::<ColorScheme>() {
        Ok(cs) => cs,
        Err(_) => {
            return ::axum::response::IntoResponse::into_response(
                ::axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    // 空文字は未設定 (None) として扱う。
    let share_url = if body.share_url.is_empty() {
        None
    } else {
        match body.share_url.parse::<ShareUrl>() {
            Ok(s) => Some(s),
            Err(_) => {
                return ::axum::response::IntoResponse::into_response(
                    ::axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                );
            }
        }
    };
    let utc_offset = match body.utc_offset.parse::<UtcOffset>() {
        Ok(o) => o,
        Err(_) => {
            return ::axum::response::IntoResponse::into_response(
                ::axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let settings = UserSettings::new(color_scheme, share_url, user_id, utc_offset);
    if let Err(e) = state.user_settings_repository.store(settings).await {
        ::tracing::error!("failed to store user settings: {e}");
        return ::axum::response::IntoResponse::into_response(
            ::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
    let redirect_url = if state.base_path.is_empty() {
        "/settings".to_string()
    } else {
        format!("{}/settings", state.base_path)
    };
    ::axum::response::IntoResponse::into_response(::axum::response::Redirect::to(&redirect_url))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;
    use crate::test_helpers::unique_user_id;

    async fn session_cookie(app: ::axum::Router, sub: &str) -> ::anyhow::Result<String> {
        let signup = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .uri("/auth/signup")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup);
        let callback = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .uri("/auth/callback?code=test&state=test_state")
                .header(::axum::http::header::COOKIE, &cookie_header)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let session = callback
            .headers()
            .get_all(::axum::http::header::SET_COOKIE)
            .iter()
            .find_map(|v| {
                let s = v.to_str().ok()?;
                if !s.contains("session") {
                    return None;
                }
                s.split(';').next().map(|p| p.to_string())
            })
            .ok_or_else(|| ::anyhow::anyhow!("session cookie not found for {sub}"))?;
        Ok(session)
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_get_settings_returns_html() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/settings")
                .header(::axum::http::header::COOKIE, session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(body.contains("Settings"));
        assert!(body.contains("data-color-scheme="));
        assert!(
            body.contains("_method=PUT"),
            "Expected form with _method=PUT, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_get_settings_requires_auth() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/settings")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_put_settings_saves_and_redirects() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::from(
                    "color_scheme=dark&utc_offset=%2B09%3A00",
                ))?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::SEE_OTHER);
        let get_response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/settings")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
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

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_put_settings_rejects_invalid_color_scheme() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::from(
                    "color_scheme=invalid&utc_offset=%2B09%3A00",
                ))?,
        )
        .await?;
        assert_eq!(
            response.status(),
            ::axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_put_settings_rejects_invalid_utc_offset() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::from(
                    "color_scheme=dark&utc_offset=invalid",
                ))?,
        )
        .await?;
        assert_eq!(
            response.status(),
            ::axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_put_settings_saves_share_url() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let body = ::serde_urlencoded::to_string([
            ("color_scheme", "dark"),
            ("share_url", "https://example.com/?u={{url}}"),
            ("utc_offset", "+09:00"),
        ])?;
        let response = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::from(body))?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::SEE_OTHER);
        let get_response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/settings")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let body = get_response.into_body_string().await?;
        assert!(
            body.contains(r#"value="https://example.com/?u={{url}}""#),
            "Expected share_url input value, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_put_settings_rejects_invalid_share_url() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let body = ::serde_urlencoded::to_string([
            ("color_scheme", "dark"),
            ("share_url", "not a url"),
            ("utc_offset", "+09:00"),
        ])?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::from(body))?,
        )
        .await?;
        assert_eq!(
            response.status(),
            ::axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_put_settings_empty_share_url_is_unset() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        // まず設定してから空で上書きし、未設定に戻ることを確認する。
        for share_url in ["https://example.com/?u={{url}}", ""] {
            let body = ::serde_urlencoded::to_string([
                ("color_scheme", "dark"),
                ("share_url", share_url),
                ("utc_offset", "+09:00"),
            ])?;
            let response = send_request(
                app.clone(),
                ::axum::http::Request::builder()
                    .method("POST")
                    .uri("/settings?_method=PUT")
                    .header(
                        ::axum::http::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .header(::axum::http::header::COOKIE, &session)
                    .body(::axum::body::Body::from(body))?,
            )
            .await?;
            assert_eq!(response.status(), ::axum::http::StatusCode::SEE_OTHER);
        }
        let get_response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/settings")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let body = get_response.into_body_string().await?;
        assert!(
            body.contains(r#"name="share_url" type="text" value=""#),
            "Expected empty share_url input, got: {body}"
        );
        assert!(
            !body.contains("{{url}}"),
            "Expected share_url to be unset, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_put_settings_requires_auth() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/settings?_method=PUT")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(::axum::body::Body::from("color_scheme=dark"))?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn test_post_without_method_override_returns_405() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/settings")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::from("color_scheme=dark"))?,
        )
        .await?;
        assert_eq!(
            response.status(),
            ::axum::http::StatusCode::METHOD_NOT_ALLOWED
        );
        Ok(())
    }
}
