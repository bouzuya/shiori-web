use crate::extractor::OidcClient;
use kernel::BookmarkReader;
use kernel::BookmarkRepository;
use kernel::UserRepository;
use kernel::UserSettingsReader;
use kernel::UserSettingsRepository;

/// `AppState` から取り出したベースパス。`CookieJar` の抽出時に使用する。
#[derive(Clone)]
pub(crate) struct BasePath(pub String);

#[derive(Clone)]
pub(crate) struct AppState {
    /// アプリケーションのベースパス (例: `/app`、空文字はルート)
    pub base_path: String,
    pub bookmark_reader: ::std::sync::Arc<dyn BookmarkReader>,
    pub bookmark_repository: ::std::sync::Arc<dyn BookmarkRepository>,
    pub cookie_key: ::axum_extra::extract::cookie::Key,
    pub oidc_client: ::std::sync::Arc<dyn OidcClient>,
    pub user_repository: ::std::sync::Arc<dyn UserRepository>,
    pub user_settings_reader: ::std::sync::Arc<dyn UserSettingsReader>,
    pub user_settings_repository: ::std::sync::Arc<dyn UserSettingsRepository>,
}

impl AppState {
    /// `cookie_signing_secret` は `Key::from()` の要件により 64 バイト以上必要。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_path: String,
        bookmark_reader: ::std::sync::Arc<dyn BookmarkReader>,
        bookmark_repository: ::std::sync::Arc<dyn BookmarkRepository>,
        cookie_signing_secret: &str,
        oidc_client: ::std::sync::Arc<dyn OidcClient>,
        user_repository: ::std::sync::Arc<dyn UserRepository>,
        user_settings_reader: ::std::sync::Arc<dyn UserSettingsReader>,
        user_settings_repository: ::std::sync::Arc<dyn UserSettingsRepository>,
    ) -> Self {
        Self {
            base_path,
            bookmark_reader,
            bookmark_repository,
            cookie_key: ::axum_extra::extract::cookie::Key::from(cookie_signing_secret.as_bytes()),
            oidc_client,
            user_repository,
            user_settings_reader,
            user_settings_repository,
        }
    }
}

impl ::axum::extract::FromRef<AppState> for BasePath {
    fn from_ref(state: &AppState) -> Self {
        BasePath(state.base_path.clone())
    }
}

impl ::axum::extract::FromRef<AppState> for ::axum_extra::extract::cookie::Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}
