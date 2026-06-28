#[derive(Clone, Debug, ::serde::Deserialize, ::serde::Serialize)]
pub(crate) struct OidcClaims {
    pub sub: String,
}

pub(crate) struct AuthenticationRequest {
    pub nonce: String,
    pub pkce_verifier: String,
    pub state: String,
    pub url: String,
}

#[::async_trait::async_trait]
pub(crate) trait OidcClient: Send + Sync {
    fn build_authentication_request(&self) -> AuthenticationRequest;
    async fn exchange_code(
        &self,
        code: &str,
        nonce: &str,
        pkce_verifier: &str,
    ) -> ::anyhow::Result<OidcClaims>;
}
