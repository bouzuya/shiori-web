use crate::OidcClaims;

// step 3 で Bearer extractor が消費するまで未使用。消費者追加時にこの allow を外す。
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
