use crate::AppState;
use crate::extractor::CurrentUserId;

pub(crate) fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(post_root))
        .route("/new", axum::routing::get(get_new))
        .route(
            "/{bookmark_id}",
            axum::routing::get(get_show)
                .patch(patch_bookmark)
                .delete(delete_bookmark)
                .post(post_bookmark_dispatch),
        )
        .route("/{bookmark_id}/delete", axum::routing::get(get_delete))
}

#[derive(askama::Template)]
#[template(path = "new.html")]
struct NewBookmarkTemplate<'a> {
    base: &'a str,
    color_scheme: &'a str,
    comment: String,
    title: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct NewBookmarkQuery {
    comment: Option<String>,
    title: Option<String>,
    url: Option<String>,
}

async fn get_new(
    CurrentUserId(user_id): CurrentUserId,
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Query(query): axum::extract::Query<NewBookmarkQuery>,
) -> impl axum::response::IntoResponse {
    let color_scheme = super::resolve_color_scheme(&state, user_id).await;
    let template = NewBookmarkTemplate {
        base: &state.base_path,
        color_scheme: &color_scheme,
        comment: query.comment.unwrap_or_default(),
        title: query.title.unwrap_or_default(),
        url: query.url.unwrap_or_default(),
    };
    match askama::Template::render(&template) {
        Ok(html) => axum::response::IntoResponse::into_response(axum::response::Html(html)),
        Err(e) => {
            tracing::error!("template render failed: {e}");
            axum::response::IntoResponse::into_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

#[derive(askama::Template)]
#[template(path = "show.html")]
struct ShowBookmarkTemplate<'a> {
    base: &'a str,
    bookmark_id: String,
    color_scheme: &'a str,
    comment: String,
    title: String,
    updated_at: String,
    url: String,
}

async fn get_show(
    CurrentUserId(user_id): CurrentUserId,
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(bookmark_id_str): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let bookmark_id = match bookmark_id_str.parse::<kernel::BookmarkId>() {
        Ok(id) => id,
        Err(_) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
    };
    let bookmark = match state.bookmark_repository.find(user_id, bookmark_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("failed to find bookmark: {e}");
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    let color_scheme = super::resolve_color_scheme(&state, user_id).await;
    let template = ShowBookmarkTemplate {
        base: &state.base_path,
        bookmark_id: bookmark_id_str,
        color_scheme: &color_scheme,
        comment: bookmark.comment().to_string(),
        title: bookmark.title().to_string(),
        updated_at: bookmark.updated_at().to_rfc3339(),
        url: bookmark.url().to_string(),
    };
    match askama::Template::render(&template) {
        Ok(html) => axum::response::IntoResponse::into_response(axum::response::Html(html)),
        Err(e) => {
            tracing::error!("template render failed: {e}");
            axum::response::IntoResponse::into_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

#[derive(askama::Template)]
#[template(path = "delete.html")]
struct DeleteBookmarkTemplate<'a> {
    base: &'a str,
    bookmark_id: String,
    color_scheme: &'a str,
    comment: String,
    title: String,
    url: String,
}

async fn get_delete(
    CurrentUserId(user_id): CurrentUserId,
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(bookmark_id_str): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let bookmark_id = match bookmark_id_str.parse::<kernel::BookmarkId>() {
        Ok(id) => id,
        Err(_) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
    };
    let bookmark = match state.bookmark_repository.find(user_id, bookmark_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("failed to find bookmark: {e}");
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    let color_scheme = super::resolve_color_scheme(&state, user_id).await;
    let template = DeleteBookmarkTemplate {
        base: &state.base_path,
        bookmark_id: bookmark_id_str,
        color_scheme: &color_scheme,
        comment: bookmark.comment().to_string(),
        title: bookmark.title().to_string(),
        url: bookmark.url().to_string(),
    };
    match askama::Template::render(&template) {
        Ok(html) => axum::response::IntoResponse::into_response(axum::response::Html(html)),
        Err(e) => {
            tracing::error!("template render failed: {e}");
            axum::response::IntoResponse::into_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

#[derive(serde::Deserialize)]
struct MethodOverrideQuery {
    #[serde(rename = "_method")]
    method: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct PatchBookmarkRequest {
    pub(crate) comment: String,
    pub(crate) title: String,
    pub(crate) updated_at: String,
    pub(crate) url: String,
}

async fn patch_bookmark_impl(
    user_id: kernel::UserId,
    state: AppState,
    bookmark_id_str: String,
    body: PatchBookmarkRequest,
) -> axum::response::Response {
    let bookmark_id = match bookmark_id_str.parse::<kernel::BookmarkId>() {
        Ok(id) => id,
        Err(_) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
    };
    let current = match state.bookmark_repository.find(user_id, bookmark_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("failed to find bookmark: {e}");
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    let url = match body.url.parse::<kernel::Url>() {
        Ok(u) => u,
        Err(_) => {
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let title = match body.title.parse::<kernel::Title>() {
        Ok(t) => t,
        Err(_) => {
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let comment = match body.comment.parse::<kernel::Comment>() {
        Ok(c) => c,
        Err(_) => {
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let updated_at = match kernel::DateTime::from_rfc3339(&body.updated_at) {
        Ok(t) => t,
        Err(_) => {
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let now = kernel::DateTime::now();
    let updated = kernel::Bookmark::new(
        comment,
        current.created_at(),
        None,
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
        return axum::response::IntoResponse::into_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
    let base = &state.base_path;
    axum::response::IntoResponse::into_response(axum::response::Redirect::to(&format!(
        "{base}/{bookmark_id_str}"
    )))
}

async fn patch_bookmark(
    CurrentUserId(user_id): CurrentUserId,
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(bookmark_id_str): axum::extract::Path<String>,
    axum::extract::Form(body): axum::extract::Form<PatchBookmarkRequest>,
) -> impl axum::response::IntoResponse {
    patch_bookmark_impl(user_id, state, bookmark_id_str, body).await
}

async fn delete_bookmark_impl(
    user_id: kernel::UserId,
    state: AppState,
    bookmark_id_str: String,
) -> axum::response::Response {
    let bookmark_id = match bookmark_id_str.parse::<kernel::BookmarkId>() {
        Ok(id) => id,
        Err(_) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
    };
    let current = match state.bookmark_repository.find(user_id, bookmark_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("failed to find bookmark: {e}");
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    let now = kernel::DateTime::now();
    let deleted = kernel::Bookmark::new(
        current.comment().clone(),
        current.created_at(),
        Some(now),
        bookmark_id,
        current.title().clone(),
        now,
        current.url().clone(),
        user_id,
    );
    if let Err(e) = state
        .bookmark_repository
        .store(Some(current.updated_at()), deleted)
        .await
    {
        tracing::error!("failed to delete bookmark: {e}");
        return axum::response::IntoResponse::into_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
    let base = &state.base_path;
    axum::response::IntoResponse::into_response(axum::response::Redirect::to(if base.is_empty() {
        "/"
    } else {
        base
    }))
}

async fn delete_bookmark(
    CurrentUserId(user_id): CurrentUserId,
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(bookmark_id_str): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    delete_bookmark_impl(user_id, state, bookmark_id_str).await
}

async fn post_bookmark_dispatch(
    CurrentUserId(user_id): CurrentUserId,
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(bookmark_id_str): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<MethodOverrideQuery>,
    body: axum::body::Bytes,
) -> impl axum::response::IntoResponse {
    match query.method.as_deref() {
        Some("PATCH") => {
            let form = match serde_urlencoded::from_bytes::<PatchBookmarkRequest>(&body) {
                Ok(f) => f,
                Err(_) => {
                    return axum::response::IntoResponse::into_response(
                        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                    );
                }
            };
            patch_bookmark_impl(user_id, state, bookmark_id_str, form).await
        }
        Some("DELETE") => delete_bookmark_impl(user_id, state, bookmark_id_str).await,
        _ => {
            axum::response::IntoResponse::into_response(axum::http::StatusCode::METHOD_NOT_ALLOWED)
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
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Form(body): axum::extract::Form<PostRootRequest>,
) -> impl axum::response::IntoResponse {
    let url = match body.url.parse::<kernel::Url>() {
        Ok(u) => u,
        Err(_) => {
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let title = match body.title.parse::<kernel::Title>() {
        Ok(t) => t,
        Err(_) => {
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let comment = match body.comment.parse::<kernel::Comment>() {
        Ok(c) => c,
        Err(_) => {
            return axum::response::IntoResponse::into_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let bookmark = kernel::Bookmark::create(user_id, url, title, comment);
    if let Err(e) = state.bookmark_repository.store(None, bookmark).await {
        tracing::error!("failed to store bookmark: {e}");
        return axum::response::IntoResponse::into_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
    let base = &state.base_path;
    axum::response::IntoResponse::into_response(axum::response::Redirect::to(if base.is_empty() {
        "/"
    } else {
        base
    }))
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::test_helpers::MockOidcClient;
    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_repo;
    use crate::test_helpers::firestore_user_settings_reader;
    use crate::test_helpers::firestore_user_settings_repository;
    use crate::test_helpers::form_body;
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;
    use crate::test_helpers::unique_user_id;

    use super::PatchBookmarkRequest;
    use super::PostRootRequest;

    async fn session_cookie(app: axum::Router, sub: &str) -> anyhow::Result<String> {
        let signup = send_request(
            app.clone(),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup);
        let callback = send_request(
            app.clone(),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test&state=test_state")
                .header(axum::http::header::COOKIE, &cookie_header)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let session = callback
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
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
            axum::http::Request::builder()
                .method("GET")
                .uri("/new")
                .header(axum::http::header::COOKIE, session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"lang="ja""#),
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
            axum::http::Request::builder()
                .method("GET")
                .uri("/new")
                .header(axum::http::header::COOKIE, session)
                .body(axum::body::Body::empty())?,
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
            axum::http::Request::builder()
                .method("GET")
                .uri("/new")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_new_with_query_params_sets_default_values() -> anyhow::Result<()> {
        let sub = format!(
            "get_new_query_params_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .method("GET")
                .uri("/new?url=https%3A%2F%2Fexample.com&title=My+Title&comment=My+Comment")
                .header(axum::http::header::COOKIE, session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"value="https://example.com""#),
            "url default value missing: {body}"
        );
        assert!(
            body.contains(r#"value="My Title""#),
            "title default value missing: {body}"
        );
        assert!(
            body.contains(r#"value="My Comment""#),
            "comment default value missing: {body}"
        );
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
            axum::http::Request::builder()
                .method("GET")
                .uri("/new")
                .header(axum::http::header::COOKIE, session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
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
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(form_body(&PostRootRequest {
                    comment: "".to_string(),
                    title: "".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
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
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, session)
                .body(form_body(&PostRootRequest {
                    comment: "my note".to_string(),
                    title: "Example".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/")
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
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "test comment".to_string(),
                    title: "Test Title".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), axum::http::StatusCode::SEE_OTHER);
        // 一覧から bookmark_id を取得
        let list_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
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
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), axum::http::StatusCode::OK);
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
            axum::http::Request::builder()
                .method("GET")
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
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
            axum::http::Request::builder()
                .method("GET")
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
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
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, session)
                .body(form_body(&PostRootRequest {
                    comment: "".to_string(),
                    title: "".to_string(),
                    url: "not-a-url".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
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
    async fn test_get_show_returns_edit_form() -> anyhow::Result<()> {
        let sub = format!(
            "get_show_edit_form_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "edit test".to_string(),
                    title: "Edit Test".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), axum::http::StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let res = send_request(
            app,
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), axum::http::StatusCode::OK);
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
            axum::http::Request::builder()
                .method("PATCH")
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(form_body(&PatchBookmarkRequest {
                    comment: "".to_string(),
                    title: "".to_string(),
                    updated_at: "2024-01-01T00:00:00.000Z".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
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
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "original".to_string(),
                    title: "Original Title".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), axum::http::StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let edit_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
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
            axum::http::Request::builder()
                .method("PATCH")
                .uri(format!("/{bookmark_id}"))
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PatchBookmarkRequest {
                    comment: "updated".to_string(),
                    title: "Updated Title".to_string(),
                    updated_at,
                    url: "https://updated.example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(
            res.headers()
                .get(axum::http::header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some(format!("/{bookmark_id}").as_str())
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_patch_via_method_override() -> anyhow::Result<()> {
        let sub = format!(
            "patch_method_override_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "original".to_string(),
                    title: "Original Title".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), axum::http::StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let show_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let show_body = show_res.into_body_string().await?;
        let updated_at = show_body
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
            .ok_or_else(|| anyhow::anyhow!("updated_at not found: {show_body}"))?;
        let res = send_request(
            app,
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/{bookmark_id}?_method=PATCH"))
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PatchBookmarkRequest {
                    comment: "updated".to_string(),
                    title: "Updated Title".to_string(),
                    updated_at,
                    url: "https://updated.example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_delete_bookmark_requires_auth() -> anyhow::Result<()> {
        let app = test_app("delete_bookmark_auth_test_user")?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .method("DELETE")
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_delete_bookmark_deletes_and_redirects() -> anyhow::Result<()> {
        let sub = format!(
            "delete_bookmark_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "to be deleted".to_string(),
                    title: "Delete Me".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), axum::http::StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("DELETE")
                .uri(format!("/{bookmark_id}"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(
            res.headers()
                .get(axum::http::header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/")
        );
        let get_res = send_request(
            app,
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(get_res.status(), axum::http::StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_delete_via_method_override() -> anyhow::Result<()> {
        let sub = format!(
            "delete_method_override_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "to be deleted".to_string(),
                    title: "Delete Me".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), axum::http::StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri(format!("/{bookmark_id}?_method=DELETE"))
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(
            res.headers()
                .get(axum::http::header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/")
        );
        let get_res = send_request(
            app,
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(get_res.status(), axum::http::StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_delete_confirm_requires_auth() -> anyhow::Result<()> {
        let app = test_app("get_delete_confirm_auth_test_user")?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .method("GET")
                .uri("/01939c78-e42a-7000-0000-000000000000/delete")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_delete_confirm_returns_404_for_unknown() -> anyhow::Result<()> {
        let sub = format!(
            "get_delete_confirm_404_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .method("GET")
                .uri("/01939c78-e42a-7000-0000-000000000000/delete")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_delete_confirm_shows_bookmark_and_delete_form() -> anyhow::Result<()> {
        let sub = format!(
            "get_delete_confirm_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let create_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "confirm delete".to_string(),
                    title: "Delete Confirm".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(create_res.status(), axum::http::StatusCode::SEE_OTHER);
        let list_res = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("GET")
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let list_body = list_res.into_body_string().await?;
        let bookmark_id = extract_bookmark_id(&list_body)?;
        let res = send_request(
            app,
            axum::http::Request::builder()
                .method("GET")
                .uri(format!("/{bookmark_id}/delete"))
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        let body = res.into_body_string().await?;
        assert!(
            body.contains("Delete Confirm"),
            "bookmark title missing: {body}"
        );
        assert!(
            body.contains(&format!(r#"action="/{bookmark_id}?_method=DELETE""#)),
            "delete form action missing: {body}"
        );
        assert!(
            body.contains(r#"method="post""#),
            "delete form method missing: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_post_without_method_override_returns_405() -> anyhow::Result<()> {
        let sub = format!(
            "post_no_method_override_user_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let app = test_app(&sub)?;
        let session = session_cookie(app.clone(), &sub).await?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .method("POST")
                .uri("/01939c78-e42a-7000-0000-000000000000")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PatchBookmarkRequest {
                    comment: "".to_string(),
                    title: "".to_string(),
                    updated_at: "2024-01-01T00:00:00.000Z".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::METHOD_NOT_ALLOWED
        );
        Ok(())
    }

    async fn session_cookie_with_base(
        app: axum::Router,
        base_path: &str,
    ) -> anyhow::Result<String> {
        let signup = send_request(
            app.clone(),
            axum::http::Request::builder()
                .uri(&format!("{base_path}/auth/signup"))
                .body(axum::body::Body::empty())?,
        )
        .await?;
        let signup_cookie = extract_cookies(&signup);
        let callback = send_request(
            app.clone(),
            axum::http::Request::builder()
                .uri(&format!(
                    "{base_path}/auth/callback?code=test_code&state=test_state"
                ))
                .header(axum::http::header::COOKIE, &signup_cookie)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        callback
            .headers()
            .get_all(axum::http::header::SET_COOKIE)
            .iter()
            .find_map(|v| {
                let s = v.to_str().ok()?;
                if !s.contains("session") {
                    return None;
                }
                s.split(';').next().map(|p| p.to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("session cookie not found"))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn with_base_path_post_root_redirects_to_base_path() -> anyhow::Result<()> {
        let base_path = "/app";
        let sub = unique_user_id();
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router(base_path).with_state(state);
        let session = session_cookie_with_base(app.clone(), base_path).await?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .method("POST")
                .uri("/app")
                .header(
                    axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(axum::http::header::COOKIE, &session)
                .body(form_body(&PostRootRequest {
                    comment: "".to_string(),
                    title: "Test".to_string(),
                    url: "https://example.com".to_string(),
                })?)?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/app")
        );
        Ok(())
    }
}
