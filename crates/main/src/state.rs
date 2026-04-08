use std::sync::Arc;

use axum_extra::extract::cookie::Key;

use crate::extractor::OidcClient;
use crate::extractor::real_oidc_client;
use crate::model::UserRepository;

#[derive(Clone)]
pub(crate) struct AppState {
    pub cookie_key: Key,
    pub oidc_client: Arc<dyn OidcClient>,
    pub user_repository: Arc<dyn UserRepository>,
}

impl AppState {
    pub fn new(oidc_client: Arc<dyn OidcClient>, user_repository: Arc<dyn UserRepository>) -> Self {
        Self {
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
        Ok(Self::new(Arc::new(oidc_client), user_repository))
    }
}

impl axum::extract::FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}
