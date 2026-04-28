use askama::Template;
use axum::extract::Form;
use axum::extract::Path;
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
        .route(
            "/{bookmark_id}",
            axum::routing::get(get_show).patch(patch_bookmark),
        )
        .route("/{bookmark_id}/edit", axum::routing::get(get_edit))
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

#[derive(Template)]
#[template(path = "bookmark.html")]
struct ShowBookmarkTemplate<'a> {
    base: &'a str,
    bookmark_id: String,
    comment: String,
    created_at: String,
    title: String,
    url: String,
}

async fn get_show(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
    Path(bookmark_id_str): Path<String>,
) -> impl IntoResponse {
    let bookmark_id = match bookmark_id_str.parse::<kernel::BookmarkId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let bookmark = match state.bookmark_repository.find(user_id, bookmark_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to find bookmark: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let template = ShowBookmarkTemplate {
        base: &state.base_path,
        bookmark_id: bookmark_id_str,
        comment: bookmark.comment().to_string(),
        created_at: bookmark.created_at().to_rfc3339(),
        title: bookmark.title().to_string(),
        url: bookmark.url().to_string(),
    };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("template render failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Template)]
#[template(path = "edit_bookmark.html")]
struct EditBookmarkTemplate<'a> {
    base: &'a str,
    bookmark_id: String,
    comment: String,
    title: String,
    updated_at: String,
    url: String,
}

async fn get_edit(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
    Path(bookmark_id_str): Path<String>,
) -> impl IntoResponse {
    let bookmark_id = match bookmark_id_str.parse::<kernel::BookmarkId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let bookmark = match state.bookmark_repository.find(user_id, bookmark_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to find bookmark: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let template = EditBookmarkTemplate {
        base: &state.base_path,
        bookmark_id: bookmark_id_str,
        comment: bookmark.comment().to_string(),
        title: bookmark.title().to_string(),
        updated_at: bookmark.updated_at().to_rfc3339(),
        url: bookmark.url().to_string(),
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
pub(crate) struct PatchBookmarkRequest {
    pub(crate) comment: String,
    pub(crate) title: String,
    pub(crate) updated_at: String,
    pub(crate) url: String,
}

async fn patch_bookmark(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
    Path(bookmark_id_str): Path<String>,
    Form(body): Form<PatchBookmarkRequest>,
) -> impl IntoResponse {
    let bookmark_id = match bookmark_id_str.parse::<kernel::BookmarkId>() {
        Ok(id) => id,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let current = match state.bookmark_repository.find(user_id, bookmark_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to find bookmark: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
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
    let updated_at = match kernel::DateTime::from_rfc3339(&body.updated_at) {
        Ok(t) => t,
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
    };
    let now = kernel::DateTime::now();
    let updated = kernel::Bookmark::new(
        comment,
        current.created_at(),
        bookmark_id,
        title,
        now,
        url,
        user_id,
    );
    if let Err(e) = state
        .bookmark_repository
        .store(Some(updated_at), updated)
        .await
    {
        tracing::error!("failed to update bookmark: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let base = &state.base_path;
    Redirect::to(&format!("{base}/{bookmark_id_str}")).into_response()
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

    use super::PatchBookmarkRequest;
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
    async fn test_get_show_returns_created_at() -> anyhow::Result<()> {
        let sub = format!(
            "get_show_created_at_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "".to_string(),
                    title: "Test Title".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = list_body
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                let marker = r#"href="/"#;
                let pos = trimmed.find(marker)?;
                let rest = &trimmed[pos + marker.len()..];
                let id = rest.split('"').next()?;
                if id.is_empty() || id.contains('/') || id.matches('-').count() != 4 {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .ok_or_else(|| anyhow::anyhow!("bookmark_id not found in list: {list_body}"))?;
        let res = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body_string().await?;
        assert!(
            body.contains(r#"class="bookmark-created-at""#),
            "created_at element missing: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_show_returns_comment() -> anyhow::Result<()> {
        let sub = format!(
            "get_show_comment_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        // ブックマークを作成
        let create_res = send_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "test comment".to_string(),
                    title: "Test Title".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), StatusCode::SEE_OTHER);
        // 一覧から bookmark_id を取得
        let list_res = send_request(
            app.clone(),
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = list_body
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                let marker = r#"href="/"#;
                let pos = trimmed.find(marker)?;
                let rest = &trimmed[pos + marker.len()..];
                let id = rest.split('"').next()?;
                if id.is_empty() || id.contains('/') || id.matches('-').count() != 4 {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .ok_or_else(|| anyhow::anyhow!("bookmark_id not found in list: {list_body}"))?;
        // 詳細ページを取得
        let res = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body_string().await?;
        assert!(body.contains("test comment"), "comment missing: {body}");
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_show_requires_auth() -> anyhow::Result<()> {
        let app = test_app("get_show_auth_test_user")?;
        let response = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_show_returns_404_for_unknown() -> anyhow::Result<()> {
        let sub = format!(
            "get_show_404_user_{}",
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
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
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

    fn extract_bookmark_id(list_body: &str) -> anyhow::Result<String> {
        list_body
            .lines()
            .find_map(|line| {
                let trimmed = line.trim();
                let marker = r#"href="/"#;
                let pos = trimmed.find(marker)?;
                let rest = &trimmed[pos + marker.len()..];
                let id = rest.split('"').next()?;
                if id.is_empty() || id.contains('/') || id.matches('-').count() != 4 {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .ok_or_else(|| anyhow::anyhow!("bookmark_id not found in list: {list_body}"))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_show_has_edit_link() -> anyhow::Result<()> {
        let sub = format!(
            "get_show_edit_link_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "".to_string(),
                    title: "Test Title".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let res = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body_string().await?;
        assert!(
            body.contains(&format!(r#"href="/{bookmark_id}/edit""#)),
            "edit link missing: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_edit_requires_auth() -> anyhow::Result<()> {
        let app = test_app("get_edit_auth_test_user")?;
        let response = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri("/01939c78-e42a-7000-0000-000000000000/edit")
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_edit_returns_404_for_unknown() -> anyhow::Result<()> {
        let sub = format!(
            "get_edit_404_user_{}",
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
                .uri("/01939c78-e42a-7000-0000-000000000000/edit")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_edit_returns_form() -> anyhow::Result<()> {
        let sub = format!(
            "get_edit_form_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "edit test".to_string(),
                    title: "Edit Test".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let res = send_request(
            app,
            Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}/edit"))
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body_string().await?;
        assert!(body.contains(r#"name="url""#), "url field missing: {body}");
        assert!(
            body.contains(r#"name="title""#),
            "title field missing: {body}"
        );
        assert!(
            body.contains(r#"name="comment""#),
            "comment field missing: {body}"
        );
        assert!(
            body.contains(r#"name="updated_at""#),
            "updated_at field missing: {body}"
        );
        assert!(body.contains("Update"), "Update button missing: {body}");
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_patch_bookmark_requires_auth() -> anyhow::Result<()> {
        let app = test_app("patch_bookmark_auth_test_user")?;
        let response = send_request(
            app,
            Request::builder()
                .method("PATCH")
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(form_body(&PatchBookmarkRequest {
                    comment: "".to_string(),
                    title: "".to_string(),
                    updated_at: "2024-01-01T00:00:00.000Z".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_patch_bookmark_updates_and_redirects() -> anyhow::Result<()> {
        let sub = format!(
            "patch_bookmark_update_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "original".to_string(),
                    title: "Original Title".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            Request::builder()
                .method("GET")
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let edit_res = send_request(
            app.clone(),
            Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}/edit"))
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        let edit_body = edit_res.into_body_string().await?;
        let updated_at = edit_body
            .lines()
            .find_map(|line| {
                let line = line.trim();
                if line.contains(r#"name="updated_at""#) {
                    let marker = r#"value=""#;
                    let pos = line.find(marker)?;
                    let rest = &line[pos + marker.len()..];
                    rest.split('"').next().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("updated_at not found in edit form: {edit_body}"))?;
        let res = send_request(
            app,
            Request::builder()
                .method("PATCH")
                .uri(format!("/{bookmark_id}"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(form_body(&PatchBookmarkRequest {
                    comment: "updated".to_string(),
                    title: "Updated Title".to_string(),
                    updated_at,
                    url: "https://updated.example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            res.headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some(format!("/{bookmark_id}").as_str())
        );
        Ok(())
    }
}
