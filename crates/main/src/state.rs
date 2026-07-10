use crate::AuthorizationCodeClient;
use crate::IdTokenVerifier;
use kernel::BookmarkReader;
use kernel::BookmarkRepository;
use kernel::UserRepository;
use kernel::UserSettingsReader;
use kernel::UserSettingsRepository;

/// `AppState` から取り出したベースパス。`CookieJar` の抽出時に使用する。
#[derive(Clone)]
pub(crate) struct BasePath(pub String);

/// CLI へ配布する OIDC クライアント設定。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CliConfig {
    pub client_id: String,
    pub client_secret: String,
    pub issuer: String,
}

#[cfg(test)]
impl CliConfig {
    pub fn for_test() -> Self {
        fn random_string() -> String {
            let mut rng = ::rand::rng();
            let len = ::rand::RngExt::random_range(&mut rng, 1..=32);
            ::rand::RngExt::sample_iter(rng, ::rand::distr::Alphanumeric)
                .take(len)
                .map(char::from)
                .collect()
        }
        Self {
            client_id: random_string(),
            client_secret: random_string(),
            issuer: format!("https://{}.example.com", random_string()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    /// アプリケーションのベースパス (例: `/app`、空文字はルート)
    pub base_path: String,
    pub bookmark_reader: ::std::sync::Arc<dyn BookmarkReader>,
    pub bookmark_repository: ::std::sync::Arc<dyn BookmarkRepository>,
    pub cli_config: CliConfig,
    pub cookie_key: ::axum_extra::extract::cookie::Key,
    pub id_token_verifier: ::std::sync::Arc<dyn IdTokenVerifier>,
    pub oidc_client: ::std::sync::Arc<dyn AuthorizationCodeClient>,
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
        cli_config: CliConfig,
        cookie_signing_secret: &str,
        id_token_verifier: ::std::sync::Arc<dyn IdTokenVerifier>,
        oidc_client: ::std::sync::Arc<dyn AuthorizationCodeClient>,
        user_repository: ::std::sync::Arc<dyn UserRepository>,
        user_settings_reader: ::std::sync::Arc<dyn UserSettingsReader>,
        user_settings_repository: ::std::sync::Arc<dyn UserSettingsRepository>,
    ) -> Self {
        Self {
            base_path,
            bookmark_reader,
            bookmark_repository,
            cli_config,
            cookie_key: ::axum_extra::extract::cookie::Key::from(cookie_signing_secret.as_bytes()),
            id_token_verifier,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_config_for_test_generates_non_empty_fields() {
        let config = CliConfig::for_test();
        assert!(!config.client_id.is_empty());
        assert!(!config.client_secret.is_empty());
        assert!(config.issuer.starts_with("https://"));
    }
}
