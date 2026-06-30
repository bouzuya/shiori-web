use crate::OidcClaims;

// verify は step 4 で /export 経由 (BearerUserId) から呼ばれるまで lib ビルドでは未使用。
#[allow(dead_code)]
#[::async_trait::async_trait]
pub(crate) trait IdTokenVerifier: Send + Sync {
    async fn verify(&self, id_token: &str) -> ::anyhow::Result<OidcClaims>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubVerifier;

    #[::async_trait::async_trait]
    impl IdTokenVerifier for StubVerifier {
        async fn verify(&self, id_token: &str) -> ::anyhow::Result<OidcClaims> {
            ::anyhow::ensure!(!id_token.is_empty(), "empty id_token");
            Ok(OidcClaims {
                sub: format!("sub-for-{id_token}"),
            })
        }
    }

    #[::tokio::test]
    async fn verify_returns_claims_with_sub() -> ::anyhow::Result<()> {
        let verifier = StubVerifier;
        let claims = verifier.verify("token-abc").await?;
        assert_eq!(claims.sub, "sub-for-token-abc");
        Ok(())
    }

    #[::tokio::test]
    async fn verify_rejects_empty_token() {
        let verifier = StubVerifier;
        assert!(verifier.verify("").await.is_err());
    }
}
