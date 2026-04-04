use super::client::AuthenticationRequest;
use super::client::OidcClaims;
use super::client::OidcClient;

pub(crate) struct RealOidcClientOptions {
    pub client_id: String,
    pub client_secret: String,
    pub issuer_url: String,
    pub redirect_uri: String,
}

pub(crate) struct RealOidcClient {
    client: openidconnect::core::CoreClient,
}

impl RealOidcClient {
    pub(crate) async fn new(options: RealOidcClientOptions) -> anyhow::Result<Self> {
        let client_id = openidconnect::ClientId::new(options.client_id);
        let client_secret = openidconnect::ClientSecret::new(options.client_secret);
        let issuer_url = openidconnect::IssuerUrl::new(options.issuer_url)?;
        let redirect_url = openidconnect::RedirectUrl::new(options.redirect_uri)?;

        let provider_metadata = openidconnect::core::CoreProviderMetadata::discover_async(
            issuer_url,
            openidconnect::reqwest::async_http_client,
        )
        .await?;

        let client = openidconnect::core::CoreClient::from_provider_metadata(
            provider_metadata,
            client_id,
            Some(client_secret),
        )
        .set_redirect_uri(redirect_url);

        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl OidcClient for RealOidcClient {
    fn build_authentication_request(&self) -> AuthenticationRequest {
        let (auth_url, csrf_state, nonce) = self
            .client
            .authorize_url(
                openidconnect::AuthenticationFlow:: <openidconnect::core::CoreResponseType> ::AuthorizationCode,
                openidconnect::CsrfToken::new_random,
                openidconnect::Nonce::new_random,
            )
            .add_scope(openidconnect::Scope::new("openid".to_string()))
            .url();

        AuthenticationRequest {
            nonce: nonce.secret().to_string(),
            state: csrf_state.secret().to_string(),
            url: auth_url.to_string(),
        }
    }

    async fn exchange_code(&self, code: &str, nonce: &str) -> anyhow::Result<OidcClaims> {
        let token_response = self
            .client
            .exchange_code(openidconnect::AuthorizationCode::new(code.to_string()))
            .request_async(openidconnect::reqwest::async_http_client)
            .await?;

        let id_token = openidconnect::TokenResponse::id_token(&token_response)
            .ok_or_else(|| anyhow::anyhow!("No ID token in response"))?;

        let nonce = openidconnect::Nonce::new(nonce.to_string());
        let claims = id_token.claims(&self.client.id_token_verifier(), &nonce)?;

        Ok(OidcClaims {
            sub: claims.subject().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn new_builds_client() -> anyhow::Result<()> {
        let options = RealOidcClientOptions {
            client_id: std::env::var("OIDC_CLIENT_ID")?,
            client_secret: std::env::var("OIDC_CLIENT_SECRET")?,
            issuer_url: std::env::var("OIDC_ISSUER_URL")?,
            redirect_uri: std::env::var("OIDC_REDIRECT_URI")?,
        };
        RealOidcClient::new(options).await?;
        Ok(())
    }
}
