use std::sync::Arc;

use axum_extra::extract::cookie::Key;

use crate::extractor::OidcClient;
use crate::extractor::real_oidc_client;
use crate::model::BookmarkRepository;
use crate::model::UserRepository;

/// `AppState` から取り出したベースパス。`CookieJar` の抽出時に使用する。
#[derive(Clone)]
pub(crate) struct BasePath(pub String);

#[derive(Clone)]
pub(crate) struct AppState {
    /// アプリケーションのベースパス (例: `/app`、空文字はルート)
    pub base_path: String,
    pub bookmark_repository: Arc<dyn BookmarkRepository>,
    pub cookie_key: Key,
    pub oidc_client: Arc<dyn OidcClient>,
    pub user_repository: Arc<dyn UserRepository>,
}

impl AppState {
    /// `cookie_signing_secret` は `Key::from()` の要件により 64 バイト以上必要。
    pub fn new(
        base_path: String,
        bookmark_repository: Arc<dyn BookmarkRepository>,
        cookie_signing_secret: &str,
        oidc_client: Arc<dyn OidcClient>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            base_path,
            bookmark_repository,
            cookie_key: Key::from(cookie_signing_secret.as_bytes()),
            oidc_client,
            user_repository,
        }
    }

    pub async fn from_env(env: &crate::env::Env) -> anyhow::Result<Self> {
        let options = real_oidc_client::RealOidcClientOptions {
            client_id: env.oidc_client_id.clone(),
            client_secret: env.oidc_client_secret.clone(),
            issuer_url: env.oidc_issuer_url.clone(),
            redirect_uri: env.oidc_redirect_uri.clone(),
        };
        let oidc_client = real_oidc_client::RealOidcClient::new(options).await?;
        let firestore = bouzuya_firestore_client::Firestore::new(
            bouzuya_firestore_client::FirestoreOptions::default(),
        )?;
        let bookmark_repository = Arc::new(crate::model::FirestoreBookmarkRepository::new(
            firestore.clone(),
        ));
        let user_repository = Arc::new(crate::model::FirestoreUserRepository::new(firestore));
        Ok(Self::new(
            env.base_path.clone(),
            bookmark_repository,
            &env.cookie_signing_secret,
            Arc::new(oidc_client),
            user_repository,
        ))
    }
}

impl axum::extract::FromRef<AppState> for BasePath {
    fn from_ref(state: &AppState) -> Self {
        BasePath(state.base_path.clone())
    }
}

impl axum::extract::FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}
