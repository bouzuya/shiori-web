use std::sync::Arc;

use axum_extra::extract::cookie::Key;

use crate::extractor::OidcClient;
use crate::extractor::real_oidc_client;
use crate::model::UserRepository;

/// `AppState` から取り出したベースパス。`CookieJar` の抽出時に使用する。
#[derive(Clone)]
pub(crate) struct BasePath(pub String);

#[derive(Clone)]
pub(crate) struct AppState {
    /// アプリケーションのベースパス (例: `/app`、空文字はルート)
    pub base_path: String,
    pub cookie_key: Key,
    pub oidc_client: Arc<dyn OidcClient>,
    pub user_repository: Arc<dyn UserRepository>,
}

impl AppState {
    pub fn new(
        base_path: String,
        oidc_client: Arc<dyn OidcClient>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            base_path,
            cookie_key: Key::generate(),
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
        let user_repository = Arc::new(crate::model::FirestoreUserRepository::new(firestore));
        Ok(Self::new(
            env.base_path.clone(),
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
