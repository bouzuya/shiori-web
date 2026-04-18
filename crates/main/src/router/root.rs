use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::AppState;
use crate::extractor::CurrentUserId;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/", get(handler))
}

#[derive(serde::Deserialize)]
struct RootQuery {
    page_token: Option<String>,
}

async fn handler(
    State(state): State<AppState>,
    auth: Option<CurrentUserId>,
    Query(query): Query<RootQuery>,
) -> impl IntoResponse {
    match auth {
        Some(CurrentUserId(user_id)) => {
            match state.bookmark_reader.list(user_id, query.page_token).await {
                Ok(list) => {
                    let body_content = if list.items.is_empty() {
                        "<p>No bookmarks</p>".to_string()
                    } else {
                        let mut sections = String::new();
                        let mut current_date = String::new();
                        let mut current_items = String::new();
                        for b in &list.items {
                            let date = b.created_at.chars().take(10).collect::<String>();
                            if date != current_date {
                                if !current_date.is_empty() {
                                    sections.push_str(&format!(
                                        "<h2>{current_date}</h2>\n<ul>\n{current_items}</ul>\n"
                                    ));
                                    current_items.clear();
                                }
                                current_date = date;
                            }
                            current_items.push_str(&format!(
                                "<li><a href=\"{}\">{}</a></li>\n",
                                b.url, b.title
                            ));
                        }
                        if !current_date.is_empty() {
                            sections.push_str(&format!(
                                "<h2>{current_date}</h2>\n<ul>\n{current_items}</ul>\n"
                            ));
                        }
                        sections
                    };
                    let base = &state.base_path;
                    Html(format!(
                        r#"<!DOCTYPE html>
<html>
<head><title>shiori</title></head>
<body>
<h1>shiori</h1>
<p><a href="{base}/new">New</a></p>
{body_content}
</body>
</html>"#
                    ))
                    .into_response()
                }
                Err(e) => {
                    tracing::error!("failed to list bookmarks: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
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

    use axum::body::Body;
    use axum::http::header;

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

    async fn session_cookie(app: axum::Router) -> anyhow::Result<String> {
        let signup = send_request(
            app.clone(),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup);
        let callback = send_request(
            app.clone(),
            axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
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
            .ok_or_else(|| anyhow::anyhow!("session cookie not found"))?;
        Ok(session)
    }

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
    async fn get_root_with_session_contains_new_link() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .uri("/")
                .header(axum::http::header::COOKIE, &session)
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("/new"),
            "Expected link to /new in root page, got: {body}"
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
            body.contains("<!DOCTYPE html"),
            "Expected HTML response, got: {body}"
        );
        assert!(
            body.contains("No bookmarks"),
            "Expected 'No bookmarks' for user with no bookmarks, got: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn get_root_with_session_and_bookmarks_returns_html_list() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let created = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(Body::from(
                    "url=https%3A%2F%2Fexample.com&title=Example+Title&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), axum::http::StatusCode::SEE_OTHER);
        let response = send_request(
            app,
            axum::http::Request::builder()
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("<!DOCTYPE html"),
            "Expected HTML, got: {body}"
        );
        assert!(
            body.contains("https://example.com"),
            "Expected bookmark URL, got: {body}"
        );
        assert!(
            body.contains("Example Title"),
            "Expected bookmark title, got: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn get_root_with_session_no_bookmarks_returns_empty_html() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let response = send_request(
            app,
            axum::http::Request::builder()
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("<!DOCTYPE html"),
            "Expected HTML, got: {body}"
        );
        assert!(
            body.contains("No bookmarks"),
            "Expected 'No bookmarks', got: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn get_root_groups_bookmarks_by_date() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let created = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(Body::from(
                    "url=https%3A%2F%2Fexample.com&title=Example+Title&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), axum::http::StatusCode::SEE_OTHER);
        let response = send_request(
            app,
            axum::http::Request::builder()
                .uri("/")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        assert!(
            body.contains(&format!("<h2>{today}</h2>")),
            "Expected date heading <h2>{today}</h2> in body, got: {body}"
        );
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn get_root_with_page_token_filters_bookmarks() -> anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        // ブックマークを1件作成
        let created = send_request(
            app.clone(),
            axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, &session)
                .body(Body::from(
                    "url=https%3A%2F%2Fexample.com&title=Example&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), axum::http::StatusCode::SEE_OTHER);
        // 過去のトークンを渡すと全件より古いものが存在しないため空になる
        let response = send_request(
            app,
            axum::http::Request::builder()
                .uri("/?page_token=2000-01-01T00:00:00.000Z")
                .header(header::COOKIE, &session)
                .body(Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("No bookmarks"),
            "Expected 'No bookmarks' when page_token filters out all items, got: {body}"
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
