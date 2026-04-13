use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::AppState;
use crate::extractor::CurrentUserId;

pub(crate) fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/bookmarks", axum::routing::post(post_bookmarks))
}

#[derive(serde::Deserialize)]
struct PostBookmarksRequest {
    comment: String,
    title: String,
    url: String,
}

#[derive(serde::Serialize)]
struct PostBookmarksResponse {
    bookmark_id: String,
}

async fn post_bookmarks(
    CurrentUserId(user_id): CurrentUserId,
    State(state): State<AppState>,
    Json(body): Json<PostBookmarksRequest>,
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
    let bookmark_id = bookmark.id().to_string();
    if let Err(e) = state.bookmark_repository.store(None, bookmark).await {
        tracing::error!("failed to store bookmark: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    (
        StatusCode::CREATED,
        Json(PostBookmarksResponse { bookmark_id }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use axum::http::header;

    use crate::model::FirestoreBookmarkRepository;
    use crate::model::FirestoreUserRepository;
    use crate::model::UserRepository;
    use crate::test_helpers::MockOidcClient;
    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::send_request;

    const TEST_COOKIE_SIGNING_SECRET: &str =
        "test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding";

    fn test_app(sub: impl Into<String>) -> anyhow::Result<axum::Router> {
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        let bookmark_repository = Arc::new(FirestoreBookmarkRepository::new(firestore.clone()));
        let user_repository: Arc<dyn UserRepository> =
            Arc::new(FirestoreUserRepository::new(firestore));
        let state = crate::AppState::new(
            "".to_string(),
            bookmark_repository,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new(sub)),
            user_repository,
        );
        Ok(crate::router::router("").with_state(state))
    }

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
    async fn test_post_bookmarks_requires_auth() -> anyhow::Result<()> {
        let app = test_app("bookmark_auth_test_user")?;
        let response = send_request(
            app,
            Request::builder()
                .method("POST")
                .uri("/bookmarks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"url":"https://example.com","title":"","comment":""}"#,
                ))?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_post_bookmarks_creates_bookmark() -> anyhow::Result<()> {
        let sub = format!(
            "bookmark_create_user_{}",
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
                .uri("/bookmarks")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, session)
                .body(Body::from(
                    r#"{"url":"https://example.com","title":"Example","comment":"my note"}"#,
                ))?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body_string().await?;
        let json: serde_json::Value = serde_json::from_str(&body)?;
        assert!(json["bookmark_id"].is_string());
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_post_bookmarks_rejects_invalid_url() -> anyhow::Result<()> {
        let sub = format!(
            "bookmark_invalid_url_user_{}",
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
                .uri("/bookmarks")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, session)
                .body(Body::from(r#"{"url":"not-a-url","title":"","comment":""}"#))?,
        )
        .await?;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        Ok(())
    }
}
