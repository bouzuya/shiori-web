use askama::Template;
use axum::extract::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Redirect;

use crate::AppState;
use crate::extractor::CurrentUserId;

pub(crate) fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(post_root))
        .route("/new", axum::routing::get(get_new))
}

#[derive(Template)]
#[template(path = "new_bookmark.html")]
struct NewBookmarkTemplate<'a> {
    base: &'a str,
}

async fn get_new(
    CurrentUserId(_user_id): CurrentUserId,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let template = NewBookmarkTemplate {
        base: &state.base_path,
    };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("template render failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct PostRootRequest {
    pub(crate) comment: String,
    pub(crate) title: String,
    pub(crate) url: String,
}

async fn post_root(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
    Form(body): Form<PostRootRequest>,
) -> impl IntoResponse {
    let url = match body.url.parse::<kernel::Url>() {
        Ok(u) => u,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    let title = match body.title.parse::<kernel::Title>() {
        Ok(t) => t,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    let comment = match body.comment.parse::<kernel::Comment>() {
        Ok(c) => c,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    let bookmark = kernel::Bookmark::create(user_id, url, title, comment);
    if let Err(e) = state.bookmark_repository.store(None, bookmark).await {
        tracing::error!("failed to store bookmark: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let base = &state.base_path;
    Redirect::to(&format!("{base}/")).into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use axum::http::header;

    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::form_body;
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;

    use super::PostRootRequest;

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
    async fn test_get_new_html_has_lang_ja_and_utf8() -> anyhow::Result<()> {
        let sub = format!(
            "get_new_lang_charset_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri("/new")
                .header(header::COOKIE, session)
                .body(Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"<html lang="ja">"#),
            "Expected lang=ja on html element, got: {body}"
        );
        assert!(
            body.contains(r#"<meta charset="UTF-8">"#),
            "Expected charset=UTF-8 meta tag, got: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_new_contains_css_link() -> anyhow::Result<()> {
        let sub = format!(
            "get_new_css_link_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri("/new")
                .header(header::COOKIE, session)
                .body(Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"rel="stylesheet""#),
            "Expected stylesheet link in /new page, got: {body}"
        );
        assert!(
            body.contains("/index.css"),
            "Expected /index.css href in /new page, got: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_new_requires_auth() -> anyhow::Result<()> {
        let app = test_app("get_new_auth_test_user")?;
        let response = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri("/new")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_new_returns_form() -> anyhow::Result<()> {
        let sub = format!(
            "get_new_form_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri("/new")
                .header(header::COOKIE, session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"action="/""#),
            "form action missing: {body}"
        );
        assert!(
            body.contains(r#"method="post""#),
            "form method missing: {body}"
        );
        assert!(body.contains(r#"name="url""#), "url field missing: {body}");
        assert!(
            body.contains(r#"name="title""#),
            "title field missing: {body}"
        );
        assert!(
            body.contains(r#"name="comment""#),
            "comment field missing: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_post_root_requires_auth() -> anyhow::Result<()> {
        let app = test_app("post_root_auth_test_user")?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(form_body(&PostRootRequest {
                    comment: "".to_string(),
                    title: "".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_post_root_creates_bookmark_and_redirects() -> anyhow::Result<()> {
        let sub = format!(
            "post_root_create_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, session)
                .body(form_body(&PostRootRequest {
                    comment: "my note".to_string(),
                    title: "Example".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/")
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_post_root_rejects_invalid_url() -> anyhow::Result<()> {
        let sub = format!(
            "post_root_invalid_url_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, session)
                .body(form_body(&PostRootRequest {
                    comment: "".to_string(),
                    title: "".to_string(),
                    url: "not-a-url".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        Ok(())
    }
}
