use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::response::IntoResponse;
use axum::response::IntoResponseParts;
use axum::response::Response;
use axum::response::ResponseParts;
use axum_extra::extract::SignedCookieJar;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::cookie::Key;

use crate::extractor::AuthenticationRequest;
use crate::extractor::OidcClaims;

pub(crate) struct CookieJar(SignedCookieJar);

impl CookieJar {
    const FLOW_COOKIE_NAME: &str = "auth_flow";
    const NONCE_COOKIE_NAME: &str = "oidc_nonce";
    const SESSION_COOKIE_NAME: &str = "session";
    const STATE_COOKIE_NAME: &str = "oidc_state";

    pub(crate) fn get_flow(&self) -> Option<String> {
        self.0
            .get(Self::FLOW_COOKIE_NAME)
            .map(|c| c.value().to_string())
    }

    pub(crate) fn get_nonce(&self) -> Option<String> {
        self.0
            .get(Self::NONCE_COOKIE_NAME)
            .map(|c| c.value().to_string())
    }

    pub(crate) fn get_session(&self) -> Option<OidcClaims> {
        self.0
            .get(Self::SESSION_COOKIE_NAME)
            .and_then(|c| serde_json::from_str::<OidcClaims>(c.value()).ok())
    }

    pub(crate) fn get_state(&self) -> Option<String> {
        self.0
            .get(Self::STATE_COOKIE_NAME)
            .map(|c| c.value().to_string())
    }

    pub(crate) fn with_session_cookies(&self, oidc_claims: OidcClaims) -> Self {
        let session_value =
            serde_json::to_string(&oidc_claims).expect("Failed to serialize session claims");

        let jar = self
            .0
            .clone()
            .remove(Cookie::from(Self::FLOW_COOKIE_NAME))
            .remove(Cookie::from(Self::STATE_COOKIE_NAME))
            .remove(Cookie::from(Self::NONCE_COOKIE_NAME))
            .add(Cookie::build((Self::SESSION_COOKIE_NAME, session_value)).path("/"));

        Self(jar)
    }

    pub(crate) fn with_signin_cookies(&self, auth_request: &AuthenticationRequest) -> Self {
        let jar = self
            .0
            .clone()
            .add(Cookie::new(Self::FLOW_COOKIE_NAME, "signin".to_string()))
            .add(Cookie::new(
                Self::NONCE_COOKIE_NAME,
                auth_request.nonce.clone(),
            ))
            .add(Cookie::new(
                Self::STATE_COOKIE_NAME,
                auth_request.state.clone(),
            ));
        Self(jar)
    }

    pub(crate) fn with_signup_cookies(&self, auth_request: &AuthenticationRequest) -> Self {
        let jar = self
            .0
            .clone()
            .add(Cookie::new(Self::FLOW_COOKIE_NAME, "signup".to_string()))
            .add(Cookie::new(
                Self::NONCE_COOKIE_NAME,
                auth_request.nonce.clone(),
            ))
            .add(Cookie::new(
                Self::STATE_COOKIE_NAME,
                auth_request.state.clone(),
            ));
        Self(jar)
    }
}

impl<S> FromRequestParts<S> for CookieJar
where
    Key: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = <SignedCookieJar as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        SignedCookieJar::from_request_parts(parts, state)
            .await
            .map(Self)
    }
}

impl IntoResponseParts for CookieJar {
    type Error = Infallible;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.0.into_response_parts(res)
    }
}

impl IntoResponse for CookieJar {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum_extra::extract::SignedCookieJar;
    use axum_extra::extract::cookie::Cookie;
    use axum_extra::extract::cookie::Key;

    use crate::extractor::AuthenticationRequest;
    use crate::extractor::OidcClaims;

    use super::CookieJar;

    fn make_empty_jar() -> CookieJar {
        let key = Key::generate();
        CookieJar(SignedCookieJar::new(key))
    }

    fn make_auth_request() -> AuthenticationRequest {
        AuthenticationRequest {
            nonce: "test_nonce".to_string(),
            state: "test_state".to_string(),
            url: "https://example.com/auth".to_string(),
        }
    }

    #[test]
    fn test_get_flow_returns_none_when_empty() {
        let jar = make_empty_jar();
        assert_eq!(jar.get_flow(), None);
    }

    #[test]
    fn test_get_nonce_returns_none_when_empty() {
        let jar = make_empty_jar();
        assert_eq!(jar.get_nonce(), None);
    }

    #[test]
    fn test_get_state_returns_none_when_empty() {
        let jar = make_empty_jar();
        assert_eq!(jar.get_state(), None);
    }

    #[test]
    fn test_get_session_returns_none_when_empty() {
        let jar = make_empty_jar();
        assert!(jar.get_session().is_none());
    }

    #[test]
    fn test_get_session_returns_none_for_invalid_json() {
        let key = Key::generate();
        let jar = CookieJar(SignedCookieJar::new(key).add(Cookie::new("session", "invalid-json")));
        assert!(jar.get_session().is_none());
    }

    #[test]
    fn test_with_signin_cookies_sets_flow() {
        let jar = make_empty_jar().with_signin_cookies(&make_auth_request());
        assert_eq!(jar.get_flow(), Some("signin".to_string()));
    }

    #[test]
    fn test_with_signin_cookies_sets_nonce() {
        let jar = make_empty_jar().with_signin_cookies(&make_auth_request());
        assert_eq!(jar.get_nonce(), Some("test_nonce".to_string()));
    }

    #[test]
    fn test_with_signin_cookies_sets_state() {
        let jar = make_empty_jar().with_signin_cookies(&make_auth_request());
        assert_eq!(jar.get_state(), Some("test_state".to_string()));
    }

    #[test]
    fn test_with_signup_cookies_sets_flow() {
        let jar = make_empty_jar().with_signup_cookies(&make_auth_request());
        assert_eq!(jar.get_flow(), Some("signup".to_string()));
    }

    #[test]
    fn test_with_signup_cookies_sets_nonce() {
        let jar = make_empty_jar().with_signup_cookies(&make_auth_request());
        assert_eq!(jar.get_nonce(), Some("test_nonce".to_string()));
    }

    #[test]
    fn test_with_signup_cookies_sets_state() {
        let jar = make_empty_jar().with_signup_cookies(&make_auth_request());
        assert_eq!(jar.get_state(), Some("test_state".to_string()));
    }

    #[test]
    fn test_with_session_cookies_sets_session() -> anyhow::Result<()> {
        let jar = make_empty_jar()
            .with_signin_cookies(&make_auth_request())
            .with_session_cookies(OidcClaims {
                sub: "user123".to_string(),
            });
        let session = jar
            .get_session()
            .ok_or_else(|| anyhow::anyhow!("expected Some(OidcClaims)"))?;
        assert_eq!(session.sub, "user123");
        Ok(())
    }

    #[test]
    fn test_with_session_cookies_removes_flow_nonce_state() {
        let jar = make_empty_jar()
            .with_signin_cookies(&make_auth_request())
            .with_session_cookies(OidcClaims {
                sub: "user123".to_string(),
            });
        assert_eq!(jar.get_flow(), None);
        assert_eq!(jar.get_nonce(), None);
        assert_eq!(jar.get_state(), None);
    }
}
