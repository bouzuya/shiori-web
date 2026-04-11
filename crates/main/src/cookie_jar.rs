use std::convert::Infallible;

use axum::extract::FromRef;
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
use crate::state::BasePath;

pub(crate) struct CookieJar {
    base_path: String,
    jar: SignedCookieJar,
}

impl CookieJar {
    const FLOW_COOKIE_NAME: &str = "auth_flow";
    const NONCE_COOKIE_NAME: &str = "oidc_nonce";
    const SESSION_COOKIE_NAME: &str = "session";
    const STATE_COOKIE_NAME: &str = "oidc_state";

    /// Cookie の `Path` 属性に設定する値を返す。
    /// `base_path` が空のときは `/`、それ以外は `base_path` そのものを使う。
    fn cookie_path(&self) -> String {
        if self.base_path.is_empty() {
            "/".to_string()
        } else {
            self.base_path.clone()
        }
    }

    pub(crate) fn get_flow(&self) -> Option<String> {
        self.jar
            .get(Self::FLOW_COOKIE_NAME)
            .map(|c| c.value().to_string())
    }

    pub(crate) fn get_nonce(&self) -> Option<String> {
        self.jar
            .get(Self::NONCE_COOKIE_NAME)
            .map(|c| c.value().to_string())
    }

    pub(crate) fn get_session(&self) -> Option<OidcClaims> {
        self.jar
            .get(Self::SESSION_COOKIE_NAME)
            .and_then(|c| serde_json::from_str::<OidcClaims>(c.value()).ok())
    }

    pub(crate) fn get_state(&self) -> Option<String> {
        self.jar
            .get(Self::STATE_COOKIE_NAME)
            .map(|c| c.value().to_string())
    }

    pub(crate) fn with_session_cookies(&self, oidc_claims: OidcClaims) -> Self {
        let session_value =
            serde_json::to_string(&oidc_claims).expect("Failed to serialize session claims");
        let cp = self.cookie_path();

        let jar = self
            .jar
            .clone()
            .remove(Cookie::build((Self::FLOW_COOKIE_NAME, "")).path(cp.clone()))
            .remove(Cookie::build((Self::STATE_COOKIE_NAME, "")).path(cp.clone()))
            .remove(Cookie::build((Self::NONCE_COOKIE_NAME, "")).path(cp.clone()))
            .add(Cookie::build((Self::SESSION_COOKIE_NAME, session_value)).path(cp));

        Self {
            base_path: self.base_path.clone(),
            jar,
        }
    }

    pub(crate) fn with_signin_cookies(&self, auth_request: &AuthenticationRequest) -> Self {
        let cp = self.cookie_path();
        let jar = self
            .jar
            .clone()
            .add(Cookie::build((Self::FLOW_COOKIE_NAME, "signin")).path(cp.clone()))
            .add(
                Cookie::build((Self::NONCE_COOKIE_NAME, auth_request.nonce.clone()))
                    .path(cp.clone()),
            )
            .add(Cookie::build((Self::STATE_COOKIE_NAME, auth_request.state.clone())).path(cp));
        Self {
            base_path: self.base_path.clone(),
            jar,
        }
    }

    pub(crate) fn with_signup_cookies(&self, auth_request: &AuthenticationRequest) -> Self {
        let cp = self.cookie_path();
        let jar = self
            .jar
            .clone()
            .add(Cookie::build((Self::FLOW_COOKIE_NAME, "signup")).path(cp.clone()))
            .add(
                Cookie::build((Self::NONCE_COOKIE_NAME, auth_request.nonce.clone()))
                    .path(cp.clone()),
            )
            .add(Cookie::build((Self::STATE_COOKIE_NAME, auth_request.state.clone())).path(cp));
        Self {
            base_path: self.base_path.clone(),
            jar,
        }
    }
}

impl<S> FromRequestParts<S> for CookieJar
where
    Key: axum::extract::FromRef<S>,
    BasePath: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = <SignedCookieJar as FromRequestParts<S>>::Rejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let jar = SignedCookieJar::from_request_parts(parts, state).await?;
        let base_path = BasePath::from_ref(state);
        Ok(Self {
            base_path: base_path.0,
            jar,
        })
    }
}

impl IntoResponseParts for CookieJar {
    type Error = Infallible;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.jar.into_response_parts(res)
    }
}

impl IntoResponse for CookieJar {
    fn into_response(self) -> Response {
        self.jar.into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum_extra::extract::SignedCookieJar;
    use axum_extra::extract::cookie::Key;

    use crate::extractor::AuthenticationRequest;
    use crate::extractor::OidcClaims;

    use super::CookieJar;

    fn make_empty_jar() -> CookieJar {
        let key = Key::generate();
        CookieJar {
            base_path: "".to_string(),
            jar: SignedCookieJar::new(key),
        }
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
        use axum_extra::extract::cookie::Cookie;
        let key = Key::generate();
        let jar = CookieJar {
            base_path: "".to_string(),
            jar: SignedCookieJar::new(key).add(Cookie::new("session", "invalid-json")),
        };
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

    #[test]
    fn test_with_session_cookies_sets_root_path_when_base_path_is_empty() -> anyhow::Result<()> {
        let jar = make_empty_jar().with_session_cookies(OidcClaims {
            sub: "user123".to_string(),
        });
        let cookie = jar
            .jar
            .get(CookieJar::SESSION_COOKIE_NAME)
            .ok_or_else(|| anyhow::anyhow!("session cookie not found"))?;
        assert_eq!(cookie.path(), Some("/"));
        Ok(())
    }

    #[test]
    fn test_with_session_cookies_sets_base_path_when_base_path_is_set() -> anyhow::Result<()> {
        let key = Key::generate();
        let jar = CookieJar {
            base_path: "/app".to_string(),
            jar: SignedCookieJar::new(key),
        };
        let jar = jar.with_session_cookies(OidcClaims {
            sub: "user123".to_string(),
        });
        let cookie = jar
            .jar
            .get(CookieJar::SESSION_COOKIE_NAME)
            .ok_or_else(|| anyhow::anyhow!("session cookie not found"))?;
        assert_eq!(cookie.path(), Some("/app"));
        Ok(())
    }

    #[test]
    fn test_with_signin_cookies_sets_root_path_when_base_path_is_empty() -> anyhow::Result<()> {
        let jar = make_empty_jar().with_signin_cookies(&make_auth_request());
        let cookie = jar
            .jar
            .get(CookieJar::FLOW_COOKIE_NAME)
            .ok_or_else(|| anyhow::anyhow!("auth_flow cookie not found"))?;
        assert_eq!(cookie.path(), Some("/"));
        Ok(())
    }

    #[test]
    fn test_with_signin_cookies_sets_base_path_when_base_path_is_set() -> anyhow::Result<()> {
        let key = Key::generate();
        let jar = CookieJar {
            base_path: "/app".to_string(),
            jar: SignedCookieJar::new(key),
        };
        let jar = jar.with_signin_cookies(&make_auth_request());
        let cookie = jar
            .jar
            .get(CookieJar::FLOW_COOKIE_NAME)
            .ok_or_else(|| anyhow::anyhow!("auth_flow cookie not found"))?;
        assert_eq!(cookie.path(), Some("/app"));
        Ok(())
    }
}
