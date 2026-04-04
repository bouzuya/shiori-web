mod client;
mod real_oidc_client;

#[cfg(test)]
pub(crate) use client::AuthenticationRequest;
pub(crate) use client::OidcClaims;
pub(crate) use client::OidcClient;

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum_extra::extract::SignedCookieJar;
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

const SESSION_COOKIE: &str = "session";

pub(crate) struct RequireAuth(pub OidcClaims);

impl<S> FromRequestParts<S> for RequireAuth
where
    Key: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        tracing::debug!(uri = %parts.uri, method = %parts.method, "RequireAuth: checking authentication");
        let jar = SignedCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                tracing::warn!("RequireAuth: failed to read signed cookie jar: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let session_cookie = jar.get(SESSION_COOKIE).ok_or_else(|| {
            tracing::info!(uri = %parts.uri, "RequireAuth: no session cookie found, returning 401");
            StatusCode::UNAUTHORIZED
        })?;
        let claims: OidcClaims = serde_json::from_str(session_cookie.value()).map_err(|e| {
            tracing::warn!("RequireAuth: failed to deserialize session cookie: {e}");
            StatusCode::UNAUTHORIZED
        })?;
        tracing::debug!(sub = %claims.sub, "RequireAuth: authenticated");
        Ok(RequireAuth(claims))
    }
}
