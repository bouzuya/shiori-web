use kernel::GoogleUserId;
use kernel::UserId;

use crate::AppState;
use crate::CookieJar;
use crate::state::BasePath;

pub(crate) struct CurrentUserId(pub UserId);

impl<S> ::axum::extract::FromRequestParts<S> for CurrentUserId
where
    BasePath: ::axum::extract::FromRef<S>,
    ::axum_extra::extract::cookie::Key: ::axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ::axum::http::StatusCode;

    async fn from_request_parts(
        parts: &mut ::axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        ::tracing::debug!(uri = %parts.uri, method = %parts.method, "CurrentUserId: checking authentication");
        let jar =
            <CookieJar as ::axum::extract::FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .map_err(|e| {
                    ::tracing::warn!("CurrentUserId: failed to read cookie jar: {e}");
                    ::axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
        let user_id_str = jar.get_session().ok_or_else(|| {
            ::tracing::warn!("CurrentUserId: failed to read session cookie");
            ::axum::http::StatusCode::UNAUTHORIZED
        })?;
        let user_id = user_id_str.parse::<UserId>().map_err(|e| {
            ::tracing::warn!("CurrentUserId: invalid user_id in session cookie: {e}");
            ::axum::http::StatusCode::UNAUTHORIZED
        })?;
        ::tracing::debug!(user_id = %user_id, "CurrentUserId: authenticated");
        Ok(CurrentUserId(user_id))
    }
}

impl<S> ::axum::extract::OptionalFromRequestParts<S> for CurrentUserId
where
    BasePath: ::axum::extract::FromRef<S>,
    ::axum_extra::extract::cookie::Key: ::axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ::axum::http::StatusCode;

    async fn from_request_parts(
        parts: &mut ::axum::http::request::Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let jar =
            <CookieJar as ::axum::extract::FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .map_err(|e| {
                    ::tracing::warn!("OptionalAuth: failed to read cookie jar: {e}");
                    ::axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?;
        let user_id = jar.get_session().and_then(|s| s.parse::<UserId>().ok());
        Ok(user_id.map(CurrentUserId))
    }
}

pub(crate) struct BearerUserId(pub UserId);

impl<S> ::axum::extract::FromRequestParts<S> for BearerUserId
where
    AppState: ::axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ::axum::http::StatusCode;

    async fn from_request_parts(
        parts: &mut ::axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = <AppState as ::axum::extract::FromRef<S>>::from_ref(state);
        let authorization = parts
            .headers
            .get(::axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(::axum::http::StatusCode::UNAUTHORIZED)?;
        let id_token = authorization
            .strip_prefix("Bearer ")
            .ok_or(::axum::http::StatusCode::UNAUTHORIZED)?;
        let claims = app_state
            .id_token_verifier
            .verify(id_token)
            .await
            .map_err(|e| {
                ::tracing::warn!("BearerUserId: id token verification failed: {e}");
                ::axum::http::StatusCode::UNAUTHORIZED
            })?;
        let google_user_id = claims.sub.parse::<GoogleUserId>().map_err(|e| {
            ::tracing::warn!("BearerUserId: invalid google user id: {e}");
            ::axum::http::StatusCode::UNAUTHORIZED
        })?;
        let user = app_state
            .user_repository
            .find_by_google_user_id(&google_user_id)
            .await
            .map_err(|e| {
                ::tracing::error!("BearerUserId: failed to find user: {e}");
                ::axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;
        match user {
            Some(user) => Ok(BearerUserId(user.id())),
            None => Err(::axum::http::StatusCode::UNAUTHORIZED),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BearerUserId;
    use crate::AppState;
    use crate::IdTokenVerifier;
    use crate::test_helpers::MockAuthorizationCodeClient;
    use crate::test_helpers::MockIdTokenVerifier;
    use crate::test_helpers::MockUserRepository;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_settings_reader;
    use crate::test_helpers::firestore_user_settings_repository;
    use crate::test_helpers::mock_id_token_verifier;
    use kernel::GoogleUserId;
    use kernel::User;
    use kernel::UserRepository;

    fn build_app_state(
        id_token_verifier: ::std::sync::Arc<dyn IdTokenVerifier>,
        user_repository: ::std::sync::Arc<dyn UserRepository>,
    ) -> ::anyhow::Result<AppState> {
        Ok(AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            id_token_verifier,
            ::std::sync::Arc::new(MockAuthorizationCodeClient::new("unused")),
            user_repository,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        ))
    }

    async fn extract(
        state: &AppState,
        request: ::axum::http::Request<::axum::body::Body>,
    ) -> Result<BearerUserId, ::axum::http::StatusCode> {
        let (mut parts, _body) = request.into_parts();
        <BearerUserId as ::axum::extract::FromRequestParts<AppState>>::from_request_parts(
            &mut parts, state,
        )
        .await
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn returns_user_id_for_valid_bearer_token() -> ::anyhow::Result<()> {
        let sub = "bearer_valid_user";
        let user = User::create(sub.parse::<GoogleUserId>()?);
        let user_id = user.id();
        let user_repository = ::std::sync::Arc::new(MockUserRepository::new());
        user_repository.store(user).await?;
        let state = build_app_state(
            ::std::sync::Arc::new(MockIdTokenVerifier::new(sub)),
            user_repository,
        )?;
        let request = ::axum::http::Request::builder()
            .header(::axum::http::header::AUTHORIZATION, "Bearer dummy-token")
            .body(::axum::body::Body::empty())?;
        let BearerUserId(extracted) = extract(&state, request)
            .await
            .map_err(|status| ::anyhow::anyhow!("unexpected rejection: {status}"))?;
        assert_eq!(extracted, user_id);
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn rejects_missing_authorization_header() -> ::anyhow::Result<()> {
        let state = build_app_state(
            mock_id_token_verifier(),
            ::std::sync::Arc::new(MockUserRepository::new()),
        )?;
        let request = ::axum::http::Request::builder().body(::axum::body::Body::empty())?;
        let result = extract(&state, request).await;
        assert_eq!(result.err(), Some(::axum::http::StatusCode::UNAUTHORIZED));
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn rejects_non_bearer_authorization_header() -> ::anyhow::Result<()> {
        let state = build_app_state(
            mock_id_token_verifier(),
            ::std::sync::Arc::new(MockUserRepository::new()),
        )?;
        let request = ::axum::http::Request::builder()
            .header(::axum::http::header::AUTHORIZATION, "Basic abc")
            .body(::axum::body::Body::empty())?;
        let result = extract(&state, request).await;
        assert_eq!(result.err(), Some(::axum::http::StatusCode::UNAUTHORIZED));
        Ok(())
    }

    #[::tokio::test]
    #[::serial_test::serial]
    async fn rejects_unknown_user() -> ::anyhow::Result<()> {
        let state = build_app_state(
            ::std::sync::Arc::new(MockIdTokenVerifier::new("unknown_bearer_user")),
            ::std::sync::Arc::new(MockUserRepository::new()),
        )?;
        let request = ::axum::http::Request::builder()
            .header(::axum::http::header::AUTHORIZATION, "Bearer dummy-token")
            .body(::axum::body::Body::empty())?;
        let result = extract(&state, request).await;
        assert_eq!(result.err(), Some(::axum::http::StatusCode::UNAUTHORIZED));
        Ok(())
    }
}
