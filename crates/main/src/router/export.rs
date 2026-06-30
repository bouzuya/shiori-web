use crate::AppState;
use crate::extractor::BearerUserId;

pub(crate) fn router() -> ::axum::Router<AppState> {
    ::axum::Router::new().route("/export", ::axum::routing::get(get_export))
}

#[derive(::serde::Serialize)]
struct ExportBookmark {
    comment: String,
    created_at: String,
    id: String,
    title: String,
    updated_at: String,
    url: String,
}

async fn get_export(
    BearerUserId(user_id): BearerUserId,
    ::axum::extract::State(state): ::axum::extract::State<AppState>,
) -> ::axum::response::Response {
    let views = match state.bookmark_reader.list_all(user_id).await {
        Ok(views) => views,
        Err(e) => {
            ::tracing::error!("failed to list all bookmarks for export: {e}");
            return ::axum::response::IntoResponse::into_response(
                ::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    };
    let mut body = String::new();
    for view in views {
        let export = ExportBookmark {
            comment: view.comment,
            created_at: view.created_at,
            id: view.id,
            title: view.title,
            updated_at: view.updated_at,
            url: view.url,
        };
        match ::serde_json::to_string(&export) {
            Ok(line) => {
                body.push_str(&line);
                body.push('\n');
            }
            Err(e) => {
                ::tracing::error!("failed to serialize bookmark for export: {e}");
                return ::axum::response::IntoResponse::into_response(
                    ::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        }
    }
    ::axum::response::IntoResponse::into_response((
        [(::axum::http::header::CONTENT_TYPE, "application/x-ndjson")],
        body,
    ))
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::test_helpers::MockAuthorizationCodeClient;
    use crate::test_helpers::MockIdTokenVerifier;
    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::extract_cookies;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_repo;
    use crate::test_helpers::firestore_user_settings_reader;
    use crate::test_helpers::firestore_user_settings_repository;
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;
    use crate::test_helpers::unique_user_id;

    /// Bearer 検証用とセッション用で同一の sub を持つアプリを組む。
    /// 固定 sub だと2回目以降の signup が既存ユーザーで 403 になり非冪等になるため、
    /// 呼び出し側は `unique_user_id()` を渡す。
    fn export_test_app(sub: &str) -> ::anyhow::Result<::axum::Router> {
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockIdTokenVerifier::new(sub)),
            ::std::sync::Arc::new(MockAuthorizationCodeClient::new(sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        Ok(crate::router::router("").with_state(state))
    }

    /// signup + callback でユーザーを作成し、セッション cookie を返す。
    async fn session_cookie(app: ::axum::Router) -> ::anyhow::Result<String> {
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
        callback
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
            .ok_or_else(|| ::anyhow::anyhow!("session cookie not found"))
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_export_requires_bearer_auth() -> ::anyhow::Result<()> {
        let response = send_request(
            test_app("export_no_auth_user")?,
            ::axum::http::Request::builder()
                .method("GET")
                .uri("/export")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_export_returns_ndjson_with_bookmarks() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let app = export_test_app(&sub)?;
        let session = session_cookie(app.clone()).await?;
        let create = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .method("POST")
                .uri("/")
                .header(
                    ::axum::http::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::from(
                    "url=https%3A%2F%2Fexample.com%2Fexport&title=Export+Me&comment=note",
                ))?,
        )
        .await?;
        assert_eq!(create.status(), ::axum::http::StatusCode::SEE_OTHER);

        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .method("GET")
                .uri("/export")
                .header(::axum::http::header::AUTHORIZATION, "Bearer dummy-token")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let content_type = response
            .headers()
            .get(::axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("application/x-ndjson"),
            "expected application/x-ndjson, got: {content_type}"
        );
        let body = response.into_body_string().await?;
        assert!(
            body.contains("https://example.com/export"),
            "url missing in export body: {body}"
        );
        assert!(
            body.contains("Export Me"),
            "title missing in export body: {body}"
        );
        assert_eq!(body.lines().count(), 1, "expected one NDJSON line: {body}");
        Ok(())
    }
}
