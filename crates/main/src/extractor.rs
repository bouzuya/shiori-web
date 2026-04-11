mod client;
pub(crate) mod real_oidc_client;

pub(crate) use client::AuthenticationRequest;
#[cfg(test)]
pub(crate) use client::OidcClaims;
pub(crate) use client::OidcClient;

use axum::extract::FromRequestParts;
use axum::extract::OptionalFromRequestParts;
use axum::http::StatusCode;
use axum_extra::extract::cookie::Key;

use crate::CookieJar;
use crate::state::BasePath;

pub(crate) struct CurrentUserId(pub crate::model::UserId);

impl<S> FromRequestParts<S> for CurrentUserId
where
    BasePath: axum::extract::FromRef<S>,
    Key: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        tracing::debug!(uri = %parts.uri, method = %parts.method, "CurrentUserId: checking authentication");
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|e| {
                tracing::warn!("CurrentUserId: failed to read cookie jar: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let user_id_str = jar.get_session().ok_or_else(|| {
            tracing::warn!("CurrentUserId: failed to read session cookie");
            StatusCode::UNAUTHORIZED
        })?;
        let user_id = user_id_str.parse::<crate::model::UserId>().map_err(|e| {
            tracing::warn!("CurrentUserId: invalid user_id in session cookie: {e}");
            StatusCode::UNAUTHORIZED
        })?;
        tracing::debug!(user_id = %user_id, "CurrentUserId: authenticated");
        Ok(CurrentUserId(user_id))
    }
}

impl<S> OptionalFromRequestParts<S> for CurrentUserId
where
    BasePath: axum::extract::FromRef<S>,
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
        let user_id = jar
            .get_session()
            .and_then(|s| s.parse::<crate::model::UserId>().ok());
        Ok(user_id.map(CurrentUserId))
    }
}
