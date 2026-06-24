use crate::AppState;
use crate::extractor::CurrentUserId;
use kernel::ColorScheme;
use kernel::DateTime;
use kernel::PageToken;

pub(crate) fn router() -> ::axum::Router<AppState> {
    ::axum::Router::new().route("/", ::axum::routing::get(handler))
}

#[derive(::serde::Deserialize)]
struct RootQuery {
    page_token: Option<String>,
}

#[derive(::askama::Template)]
#[template(path = "landing.html")]
struct LandingTemplate<'a> {
    base: &'a str,
    color_scheme: &'a str,
    version: &'a str,
}

struct BookmarkItem {
    id: String,
    share_url: Option<String>,
    title: String,
    url: String,
}

struct DateGroup {
    date: String,
    items: Vec<BookmarkItem>,
}

#[derive(::askama::Template)]
#[template(path = "list.html")]
struct BookmarksTemplate<'a> {
    base: &'a str,
    color_scheme: &'a str,
    groups: Vec<DateGroup>,
    next_page_token: Option<String>,
    prev_page_token: Option<String>,
    version: &'a str,
}

fn render_template(html: Result<String, ::askama::Error>) -> ::axum::response::Response {
    match html {
        Ok(html) => ::axum::response::IntoResponse::into_response(::axum::response::Html(html)),
        Err(e) => {
            ::tracing::error!("template render failed: {e}");
            ::axum::response::IntoResponse::into_response(
                ::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

async fn handler(
    ::axum::extract::State(state): ::axum::extract::State<AppState>,
    auth: Option<CurrentUserId>,
    ::axum::extract::Query(query): ::axum::extract::Query<RootQuery>,
) -> impl ::axum::response::IntoResponse {
    match auth {
        Some(CurrentUserId(user_id)) => {
            let color_scheme = super::resolve_color_scheme(&state, user_id).await;
            let offset = super::resolve_utc_offset(&state, user_id).await;
            let share_url = super::resolve_share_url(&state, user_id).await;
            let page_token = match query.page_token {
                None => None,
                Some(s) => match s.parse::<PageToken>() {
                    Ok(token) => Some(token),
                    Err(_) => {
                        return ::axum::response::IntoResponse::into_response(
                            ::axum::http::StatusCode::BAD_REQUEST,
                        );
                    }
                },
            };
            match state.bookmark_reader.list(user_id, page_token).await {
                Ok(list) => {
                    let mut groups: Vec<DateGroup> = Vec::new();
                    for b in &list.items {
                        let date = match DateTime::from_rfc3339(&b.created_at) {
                            Ok(dt) => dt.to_date_string_in(offset),
                            // 不正値は従来どおり先頭10文字でフォールバック
                            Err(_) => b.created_at.chars().take(10).collect::<String>(),
                        };
                        let item = BookmarkItem {
                            id: b.id.clone(),
                            share_url: share_url
                                .as_ref()
                                .map(|s| s.build(&b.comment, &b.title, &b.url)),
                            title: b.title.clone(),
                            url: b.url.clone(),
                        };
                        match groups.last_mut() {
                            Some(g) if g.date == date => {
                                g.items.push(item);
                            }
                            _ => {
                                groups.push(DateGroup {
                                    date,
                                    items: vec![item],
                                });
                            }
                        }
                    }
                    let template = BookmarksTemplate {
                        base: &state.base_path,
                        color_scheme: &color_scheme,
                        groups,
                        next_page_token: list.next_page_token,
                        prev_page_token: list.prev_page_token,
                        version: env!("CARGO_PKG_VERSION"),
                    };
                    render_template(::askama::Template::render(&template))
                }
                Err(e) => {
                    ::tracing::error!("failed to list bookmarks: {e}");
                    ::axum::response::IntoResponse::into_response(
                        ::axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            }
        }
        None => {
            let color_scheme = ColorScheme::default().to_string();
            let template = LandingTemplate {
                base: &state.base_path,
                color_scheme: &color_scheme,
                version: env!("CARGO_PKG_VERSION"),
            };
            render_template(::askama::Template::render(&template))
        }
    }
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
    use crate::test_helpers::send_request;
    use crate::test_helpers::test_app;
    use crate::test_helpers::unique_user_id;
    use kernel::Bookmark;
    use kernel::BookmarkId;
    use kernel::ColorScheme;
    use kernel::Comment;
    use kernel::DateTime;
    use kernel::GoogleUserId;
    use kernel::ShareUrl;
    use kernel::Title;
    use kernel::Url;
    use kernel::UserSettings;
    use kernel::UtcOffset;

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
                .uri("/auth/callback?code=test_code&state=test_state")
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
            .ok_or_else(|| ::anyhow::anyhow!("session cookie not found"))?;
        Ok(session)
    }

    async fn session_cookie_with_base(
        app: ::axum::Router,
        base_path: &str,
    ) -> ::anyhow::Result<String> {
        let signup = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .uri(format!("{base_path}/auth/signup"))
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let cookie_header = extract_cookies(&signup);
        let callback = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .uri(format!(
                    "{base_path}/auth/callback?code=test_code&state=test_state"
                ))
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
    async fn get_root_without_session_returns_landing_page() -> ::anyhow::Result<()> {
        let response = send_request(
            test_app("test_root_no_session_user")?,
            ::axum::http::Request::builder()
                .uri("/")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
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

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_with_session_contains_new_link() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("/new"),
            "Expected link to /new in root page, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_with_session_returns_ok() -> ::anyhow::Result<()> {
        // Full flow: signup → callback → access root
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );

        // Step 1: Signup
        let signup_response = send_request(
            crate::router::router("").with_state(state.clone()),
            ::axum::http::Request::builder()
                .uri("/auth/signup")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let signup_cookie_header = extract_cookies(&signup_response);

        // Step 2: Callback
        let callback_response = send_request(
            crate::router::router("").with_state(state.clone()),
            ::axum::http::Request::builder()
                .uri("/auth/callback?code=test_code&state=test_state")
                .header(::axum::http::header::COOKIE, &signup_cookie_header)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let session_cookie_header = extract_cookies(&callback_response);

        // Step 3: Access root with session cookie
        let response = send_request(
            crate::router::router("").with_state(state),
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session_cookie_header)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
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

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_with_session_and_bookmarks_returns_html_list() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let created = send_request(
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
                    "url=https%3A%2F%2Fexample.com&title=Example+Title&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
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

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_with_session_no_bookmarks_returns_empty_html() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
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

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_groups_bookmarks_by_date() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let created = send_request(
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
                    "url=https%3A%2F%2Fexample.com&title=Example+Title&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        let today = DateTime::now()
            .to_rfc3339()
            .to_string()
            .chars()
            .take(10)
            .collect::<String>();
        assert!(
            body.contains(&format!("<h2 class=\"bookmark-group-name\">{today}</h2>")),
            "Expected date heading <h2 class=\"bookmark-group-name\">{today}</h2> in body, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_groups_bookmarks_by_date_in_user_utc_offset() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state.clone());
        let session = session_cookie(app.clone()).await?;

        // 認証済みユーザーの user_id を取得する。
        let user_id = state
            .user_repository
            .find_by_google_user_id(&sub.parse::<GoogleUserId>()?)
            .await?
            .ok_or_else(|| ::anyhow::anyhow!("user not found"))?
            .id();

        // +09:00 を保存する。
        state
            .user_settings_repository
            .store(UserSettings::new(
                ColorScheme::default(),
                None,
                user_id,
                UtcOffset::new(540)?,
            ))
            .await?;

        // UTC では 2024-01-15 だが +09:00 では 2024-01-16 になる固定時刻で保存する。
        state
            .bookmark_repository
            .store(
                None,
                Bookmark::new(
                    "".parse::<Comment>()?,
                    DateTime::from_rfc3339("2024-01-15T20:00:00.000Z")?,
                    None,
                    BookmarkId::new(),
                    "Example Title".parse::<Title>()?,
                    DateTime::from_rfc3339("2024-01-15T20:00:00.000Z")?,
                    "https://example.com".parse::<Url>()?,
                    user_id,
                ),
            )
            .await?;

        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("<h2 class=\"bookmark-group-name\">2024-01-16</h2>"),
            "Expected date heading converted to +09:00 (2024-01-16), got: {body}"
        );
        assert!(
            !body.contains("<h2 class=\"bookmark-group-name\">2024-01-15</h2>"),
            "Expected UTC date heading (2024-01-15) to be absent, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_shows_share_link_when_share_url_is_set() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state.clone());
        let session = session_cookie(app.clone()).await?;
        let user_id = state
            .user_repository
            .find_by_google_user_id(&sub.parse::<GoogleUserId>()?)
            .await?
            .ok_or_else(|| ::anyhow::anyhow!("user not found"))?
            .id();

        state
            .user_settings_repository
            .store(UserSettings::new(
                ColorScheme::default(),
                Some("https://example.com/share?u={{url}}".parse::<ShareUrl>()?),
                user_id,
                UtcOffset::default(),
            ))
            .await?;

        let created = send_request(
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
                    "url=https%3A%2F%2Fexample.com&title=Example+Title&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);

        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            body.contains("https://example.com/share?u=https%3A%2F%2Fexample.com"),
            "Expected Share link with encoded url, got: {body}"
        );
        assert!(
            body.contains(">Share</a>"),
            "Expected Share menu item, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_delete_menu_links_to_delete_confirm() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let created = send_request(
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
                    "url=https%3A%2F%2Fexample.com&title=Example+Title&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            body.contains("/delete\">Delete</a>"),
            "Expected Delete menu to link to delete confirm page, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_has_no_share_link_without_share_url() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let created = send_request(
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
                    "url=https%3A%2F%2Fexample.com&title=Example+Title&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            !body.contains(">Share</a>"),
            "Expected no Share menu item when share_url is unset, got: {body}"
        );
        Ok(())
    }

    /// HTML 本文から最初の `?page_token=<不透明トークン>` の値を取り出す。
    fn extract_page_token(body: &str) -> ::anyhow::Result<String> {
        let marker = "?page_token=";
        let start = body
            .find(marker)
            .ok_or_else(|| ::anyhow::anyhow!("page token link not found"))?
            + marker.len();
        let rest = &body[start..];
        let end = rest
            .find('"')
            .ok_or_else(|| ::anyhow::anyhow!("malformed page token link"))?;
        Ok(rest[..end].to_string())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_with_page_token_returns_next_page() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        // PAGE_SIZE (10) を超える 11 件を作成し、2 ページ目に最古の 1 件が来るようにする
        for i in 0..11 {
            let created = send_request(
                app.clone(),
                ::axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header(
                        ::axum::http::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .header(::axum::http::header::COOKIE, &session)
                    .body(::axum::body::Body::from(format!(
                        "url=https%3A%2F%2Fexample.com%2F{i}&title=Title-{i:02}&comment="
                    )))?,
            )
            .await?;
            assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        }
        // 先頭ページを取得し、Next リンクの不透明トークンを取り出す
        let first = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let first_body = first.into_body_string().await?;
        // 最古の Title-00 は 1 ページ目には出ない
        assert!(
            !first_body.contains("Title-00"),
            "Expected oldest item NOT on first page, got: {first_body}"
        );
        let token = extract_page_token(&first_body)?;
        // 2 ページ目を取得すると最古の Title-00 が現れる
        let second = send_request(
            app,
            ::axum::http::Request::builder()
                .uri(format!("/?page_token={token}"))
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(second.status(), ::axum::http::StatusCode::OK);
        let second_body = second.into_body_string().await?;
        assert!(
            second_body.contains("Title-00"),
            "Expected oldest item Title-00 on second page, got: {second_body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_prev_page_link_returns_to_first_page() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        // PAGE_SIZE (10) を超える 11 件を作成し、2 ページに分かれるようにする
        for i in 0..11 {
            let created = send_request(
                app.clone(),
                ::axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header(
                        ::axum::http::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .header(::axum::http::header::COOKIE, &session)
                    .body(::axum::body::Body::from(format!(
                        "url=https%3A%2F%2Fexample.com%2F{i}&title=Title-{i:02}&comment="
                    )))?,
            )
            .await?;
            assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        }
        // 先頭ページ -> Next リンクの不透明トークン
        let first = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let first_body = first.into_body_string().await?;
        let next_token = extract_page_token(&first_body)?;
        // 2 ページ目を取得。最古の Title-00 のみが表示され、Prev リンクだけが出る
        let second = send_request(
            app.clone(),
            ::axum::http::Request::builder()
                .uri(format!("/?page_token={next_token}"))
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let second_body = second.into_body_string().await?;
        assert!(
            second_body.contains("Title-00"),
            "Expected oldest item Title-00 on second page, got: {second_body}"
        );
        // 2 ページ目には Next リンクが無く、Prev リンクだけが存在する
        let prev_token = extract_page_token(&second_body)?;
        // Prev リンクで先頭ページへ戻れる
        let back = send_request(
            app,
            ::axum::http::Request::builder()
                .uri(format!("/?page_token={prev_token}"))
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(back.status(), ::axum::http::StatusCode::OK);
        let back_body = back.into_body_string().await?;
        assert!(
            back_body.contains("Title-10") && !back_body.contains("Title-00"),
            "Expected to return to the first page, got: {back_body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_with_invalid_page_token_returns_bad_request() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/?page_token=not-a-valid-token")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_with_next_page_contains_next_page_link() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        // PAGE_SIZE (10) 件を超える 11 件を作成し、次ページが存在する状態にする
        for i in 0..11 {
            let created = send_request(
                app.clone(),
                ::axum::http::Request::builder()
                    .method("POST")
                    .uri("/")
                    .header(
                        ::axum::http::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .header(::axum::http::header::COOKIE, &session)
                    .body(::axum::body::Body::from(format!(
                        "url=https%3A%2F%2Fexample.com%2F{i}&title=Example+{i}&comment="
                    )))?,
            )
            .await?;
            assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        }
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains("?page_token="),
            "Expected next page link with ?page_token= in body, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn with_base_path_next_page_link_has_no_trailing_slash() -> ::anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(unique_user_id())),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router(base_path).with_state(state);
        let session = session_cookie_with_base(app.clone(), base_path).await?;
        // PAGE_SIZE (10) 件を超える 11 件を作成し、次ページが存在する状態にする
        for i in 0..11 {
            let created = send_request(
                app.clone(),
                ::axum::http::Request::builder()
                    .method("POST")
                    .uri("/app")
                    .header(
                        ::axum::http::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    )
                    .header(::axum::http::header::COOKIE, &session)
                    .body(::axum::body::Body::from(format!(
                        "url=https%3A%2F%2Fexample.com%2F{i}&title=Example+{i}&comment="
                    )))?,
            )
            .await?;
            assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        }
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/app")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"href="/app?page_token="#),
            "Expected next page link to be /app?page_token= (no trailing slash), got: {body}"
        );
        assert!(
            !body.contains(r#"href="/app/?page_token="#),
            "Expected no next page link with trailing slash /app/?page_token=, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_without_next_page_has_no_next_page_link() -> ::anyhow::Result<()> {
        let sub = unique_user_id();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new(&sub)),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let app = crate::router::router("").with_state(state);
        let session = session_cookie(app.clone()).await?;
        let created = send_request(
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
                    "url=https%3A%2F%2Fexample.com&title=Example&comment=",
                ))?,
        )
        .await?;
        assert_eq!(created.status(), ::axum::http::StatusCode::SEE_OTHER);
        let response = send_request(
            app,
            ::axum::http::Request::builder()
                .uri("/")
                .header(::axum::http::header::COOKIE, &session)
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            !body.contains("?page_token="),
            "Expected no next page link when there is no next page, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_html_has_lang_ja_and_utf8() -> ::anyhow::Result<()> {
        let response = send_request(
            test_app("root_lang_charset_user")?,
            ::axum::http::Request::builder()
                .uri("/")
                .body(::axum::body::Body::empty())?,
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

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_landing_has_default_color_scheme() -> ::anyhow::Result<()> {
        let response = send_request(
            test_app("root_color_scheme_user")?,
            ::axum::http::Request::builder()
                .uri("/")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"data-color-scheme="system""#),
            "Expected default data-color-scheme=system on landing, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_root_contains_css_link() -> ::anyhow::Result<()> {
        let response = send_request(
            test_app("root_css_link_user")?,
            ::axum::http::Request::builder()
                .uri("/")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"rel="stylesheet""#),
            "Expected stylesheet link in root page, got: {body}"
        );
        assert!(
            body.contains("/index.css"),
            "Expected /index.css href in root page, got: {body}"
        );
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn with_base_path_root_contains_base_path_links() -> ::anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new("base_path_links_user")),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let response = send_request(
            crate::router::router(base_path).with_state(state),
            ::axum::http::Request::builder()
                .uri("/app")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
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

    #[::tokio::test]
    #[::serial_test::serial]
    async fn with_base_path_root_link_has_no_trailing_slash() -> ::anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockOidcClient::new("base_path_trailing_slash_user")),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let response = send_request(
            crate::router::router(base_path).with_state(state),
            ::axum::http::Request::builder()
                .uri("/app")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let body = response.into_body_string().await?;
        assert!(
            body.contains(r#"href="/app""#),
            "Expected site title link to be /app (no trailing slash), got: {body}"
        );
        assert!(
            !body.contains(r#"href="/app/""#),
            "Expected no href with trailing slash /app/, got: {body}"
        );
        Ok(())
    }
}
