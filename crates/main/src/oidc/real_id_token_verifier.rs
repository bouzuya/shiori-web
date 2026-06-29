use crate::IdTokenVerifier;
use crate::OidcClaims;

// step 3 / step 4 で main が構築するまで未使用。構築側追加時にこれらの allow を外す。
#[allow(dead_code)]
pub(crate) struct RealIdTokenVerifierOptions {
    pub client_id: String,
    pub issuer_url: String,
}

#[allow(dead_code)]
pub(crate) struct RealIdTokenVerifier {
    client: ::openidconnect::core::CoreClient,
}

impl RealIdTokenVerifier {
    #[allow(dead_code)]
    pub(crate) async fn new(options: RealIdTokenVerifierOptions) -> ::anyhow::Result<Self> {
        let client_id = ::openidconnect::ClientId::new(options.client_id);
        let issuer_url = ::openidconnect::IssuerUrl::new(options.issuer_url)?;

        let provider_metadata = ::openidconnect::core::CoreProviderMetadata::discover_async(
            issuer_url,
            ::openidconnect::reqwest::async_http_client,
        )
        .await?;

        // 検証専用クライアントなので client_secret は持たない (audience = client_id で aud を検証する)。
        let client = ::openidconnect::core::CoreClient::from_provider_metadata(
            provider_metadata,
            client_id,
            None,
        );

        Ok(Self { client })
    }
}

#[::async_trait::async_trait]
impl IdTokenVerifier for RealIdTokenVerifier {
    async fn verify(&self, id_token: &str) -> ::anyhow::Result<OidcClaims> {
        let id_token = id_token.parse::<::openidconnect::core::CoreIdToken>()?;
        // Bearer として受け取った ID トークンには nonce 起点が無いため nonce 検証はスキップし、
        // 署名・iss・aud・exp の検証のみ行う。
        let claims = id_token.claims(
            &self.client.id_token_verifier(),
            |_nonce: Option<&::openidconnect::Nonce>| -> Result<(), String> { Ok(()) },
        )?;
        Ok(OidcClaims {
            sub: claims.subject().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::tokio::test]
    #[ignore]
    async fn new_builds_verifier() -> ::anyhow::Result<()> {
        let options = RealIdTokenVerifierOptions {
            client_id: ::std::env::var("OIDC_CLI_CLIENT_ID")?,
            issuer_url: ::std::env::var("OIDC_ISSUER_URL")?,
        };
        RealIdTokenVerifier::new(options).await?;
        Ok(())
    }
}
