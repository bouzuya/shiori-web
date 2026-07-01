// login が消費するまで bin ビルドでは未使用。消費側 (次の単位) を追加したら外す。
#![allow(dead_code)]

/// 認可コードフロー (loopback + PKCE) の認可リクエスト。
///
/// `authorization_url` をブラウザで開き、コールバックで受け取った認可コードを
/// `pkce_verifier` と共にトークンエンドポイントへ送って交換する。
/// `csrf_state` はコールバックの `state` と突き合わせて改ざんを検査する。
pub(crate) struct AuthorizationRequest {
    pub authorization_url: String,
    pub csrf_state: String,
    pub pkce_verifier: String,
}

/// Google の認可エンドポイント向けに、PKCE 付きの認可リクエストを組み立てる。
///
/// public client (CLI) のため client_secret は持たず、PKCE で保護する。
/// refresh token を得るため `access_type=offline` / `prompt=consent` を付ける。
pub(crate) fn build_authorization_request(
    auth_endpoint: &str,
    client_id: &str,
    redirect_uri: &str,
) -> ::anyhow::Result<AuthorizationRequest> {
    let (pkce_challenge, pkce_verifier) = ::openidconnect::PkceCodeChallenge::new_random_sha256();
    let csrf_state = ::openidconnect::CsrfToken::new_random();

    let mut url = ::url::Url::parse(auth_endpoint)?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email")
        .append_pair("state", csrf_state.secret())
        .append_pair("code_challenge", pkce_challenge.as_str())
        .append_pair("code_challenge_method", "S256")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");

    Ok(AuthorizationRequest {
        authorization_url: url.to_string(),
        csrf_state: csrf_state.secret().to_string(),
        pkce_verifier: pkce_verifier.secret().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query_params(url: &str) -> ::anyhow::Result<::std::collections::HashMap<String, String>> {
        Ok(::url::Url::parse(url)?.query_pairs().into_owned().collect())
    }

    #[test]
    fn builds_authorization_url_with_required_params_and_pkce() -> ::anyhow::Result<()> {
        let request = build_authorization_request(
            "https://accounts.google.com/o/oauth2/v2/auth",
            "client-123",
            "http://127.0.0.1:9876/callback",
        )?;
        let url = ::url::Url::parse(&request.authorization_url)?;
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("accounts.google.com"));
        assert_eq!(url.path(), "/o/oauth2/v2/auth");

        let params = query_params(&request.authorization_url)?;
        assert_eq!(
            params.get("client_id").map(String::as_str),
            Some("client-123")
        );
        assert_eq!(
            params.get("redirect_uri").map(String::as_str),
            Some("http://127.0.0.1:9876/callback")
        );
        assert_eq!(
            params.get("response_type").map(String::as_str),
            Some("code")
        );
        assert_eq!(
            params.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert_eq!(
            params.get("access_type").map(String::as_str),
            Some("offline")
        );
        assert_eq!(params.get("prompt").map(String::as_str), Some("consent"));
        assert!(
            params
                .get("scope")
                .is_some_and(|s| s.contains("openid") && s.contains("email"))
        );
        assert!(params.get("code_challenge").is_some_and(|s| !s.is_empty()));
        // state は返り値の csrf_state と一致する
        assert_eq!(
            params.get("state").map(String::as_str),
            Some(request.csrf_state.as_str())
        );
        assert!(!request.pkce_verifier.is_empty());
        Ok(())
    }

    #[test]
    fn generates_distinct_pkce_verifier_and_state_per_call() -> ::anyhow::Result<()> {
        let a =
            build_authorization_request("https://e.example/auth", "c", "http://127.0.0.1:1/cb")?;
        let b =
            build_authorization_request("https://e.example/auth", "c", "http://127.0.0.1:1/cb")?;
        assert_ne!(a.pkce_verifier, b.pkce_verifier);
        assert_ne!(a.csrf_state, b.csrf_state);
        Ok(())
    }
}
