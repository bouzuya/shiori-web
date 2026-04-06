mod client;
pub(crate) mod real_oidc_client;

pub(crate) use client::AuthenticationRequest;
pub(crate) use client::OidcClaims;
pub(crate) use client::OidcClient;

use axum::extract::FromRequestParts;
use axum::extract::OptionalFromRequestParts;
use axum::http::StatusCode;
use axum_extra::extract::cookie::Key;

use crate::CookieJar;

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
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                tracing::warn!("RequireAuth: failed to read cookie jar: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let claims = jar.get_session().ok_or_else(|| {
            tracing::warn!("RequireAuth: failed to read session cookie");
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
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                tracing::warn!("OptionalAuth: failed to read cookie jar: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let claims = jar.get_session();
        Ok(claims.map(RequireAuth))
    }
}
