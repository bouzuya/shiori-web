mod client;
mod real_oidc_client;

pub(crate) use client::{AuthenticationRequest, OidcClaims, OidcClient};

use std::sync::Arc;

use axum_extra::extract::cookie::Key;

#[derive(Clone)]
pub(crate) struct AppState {
    pub cookie_key: Key,
    pub oidc_client: Arc<dyn OidcClient>,
}

impl AppState {
    pub fn new(oidc_client: Arc<dyn OidcClient>) -> Self {
        Self {
            cookie_key: Key::generate(),
            oidc_client,
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
        Ok(Self::new(Arc::new(oidc_client)))
    }
}

impl axum::extract::FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}
