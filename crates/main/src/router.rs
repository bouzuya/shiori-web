mod auth;
mod bookmark;
mod favicon;
mod index_css;
mod root;
mod settings;

use crate::AppState;
use kernel::ColorScheme;
use kernel::ShareUrl;
use kernel::UserId;
use kernel::UtcOffset;

pub(crate) fn router(base_path: &str) -> ::axum::Router<AppState> {
    let inner = ::axum::Router::new()
        .merge(auth::router())
        .merge(bookmark::router())
        .merge(favicon::router())
        .merge(index_css::router())
        .merge(root::router())
        .merge(settings::router());
    if base_path.is_empty() {
        inner
    } else {
        ::axum::Router::new().nest(base_path, inner)
    }
}

/// 現在ユーザーの配色設定 (`data-color-scheme` 属性値) を解決する。
/// 未保存・取得失敗時は既定値 (`system`) にフォールバックする。
pub(crate) async fn resolve_color_scheme(state: &AppState, user_id: UserId) -> String {
    match state.user_settings_reader.get(user_id).await {
        Ok(Some(view)) => view.color_scheme,
        Ok(None) => ColorScheme::default().to_string(),
        Err(e) => {
            ::tracing::error!("failed to get user settings: {e}");
            ColorScheme::default().to_string()
        }
    }
}

/// 現在ユーザーの共有 URL テンプレートを解決する。
/// 未保存・未設定・取得失敗・パース失敗時は `None` を返す。
pub(crate) async fn resolve_share_url(state: &AppState, user_id: UserId) -> Option<ShareUrl> {
    match state.user_settings_reader.get(user_id).await {
        Ok(Some(view)) => view.share_url.and_then(|s| s.parse::<ShareUrl>().ok()),
        Ok(None) => None,
        Err(e) => {
            ::tracing::error!("failed to get user settings: {e}");
            None
        }
    }
}

/// 現在ユーザーの UTC オフセットを解決する。
/// 未保存・取得失敗・パース失敗時は既定値 (UTC) にフォールバックする。
pub(crate) async fn resolve_utc_offset(state: &AppState, user_id: UserId) -> UtcOffset {
    match state.user_settings_reader.get(user_id).await {
        Ok(Some(view)) => view.utc_offset.parse::<UtcOffset>().unwrap_or_default(),
        Ok(None) => UtcOffset::default(),
        Err(e) => {
            ::tracing::error!("failed to get user settings: {e}");
            UtcOffset::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::test_helpers::MockAuthorizationCodeClient;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_repo;
    use crate::test_helpers::firestore_user_settings_reader;
    use crate::test_helpers::firestore_user_settings_repository;
    use crate::test_helpers::send_request;

    #[::tokio::test]
    #[::serial_test::serial]
    async fn with_base_path_routes_are_under_base_path() -> ::anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            ::std::sync::Arc::new(MockAuthorizationCodeClient::new("base_path_route_user")),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );

        // Route exists under base path
        let response = send_request(
            super::router(base_path).with_state(state.clone()),
            ::axum::http::Request::builder()
                .uri("/app/auth/signup")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            ::axum::http::StatusCode::TEMPORARY_REDIRECT,
            "Expected route under base path to exist"
        );

        // Route does NOT exist without base path
        let response = send_request(
            super::router(base_path).with_state(state),
            ::axum::http::Request::builder()
                .uri("/auth/signup")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            ::axum::http::StatusCode::NOT_FOUND,
            "Expected route without base path to return 404"
        );
        Ok(())
    }
}
