mod client;
pub(crate) mod real_oidc_client;

#[cfg(test)]
pub(crate) use client::AuthenticationRequest;
pub(crate) use client::OidcClaims;
pub(crate) use client::OidcClient;

use axum::extract::FromRequestParts;
use axum::extract::OptionalFromRequestParts;
use axum::http::StatusCode;
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Key;

use crate::cookie::SESSION_COOKIE;

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

impl<S> OptionalFromRequestParts<S> for RequireAuth
where
    Key: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let jar = SignedCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                tracing::warn!("OptionalAuth: failed to read signed cookie jar: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let claims = jar
            .get(SESSION_COOKIE)
            .and_then(|c| serde_json::from_str::<OidcClaims>(c.value()).ok());
        Ok(claims.map(RequireAuth))
    }
}
