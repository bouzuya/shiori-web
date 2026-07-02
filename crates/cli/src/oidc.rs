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

/// トークンエンドポイントの応答 (必要なフィールドのみ。未知フィールドは無視)。
#[derive(Clone, Debug, Eq, PartialEq, ::serde::Deserialize)]
pub(crate) struct TokenResponse {
    pub id_token: String,
    pub refresh_token: Option<String>,
}

/// 認可コードを refresh_token / id_token へ交換するためのパラメータ。
///
/// Google のデスクトップ型クライアントはトークン交換に client_secret を要求するが、
/// これは「秘密として扱わない」見せかけの secret で、実際の保護は PKCE の code_verifier が担う。
pub(crate) struct TokenExchange<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub code: &'a str,
    pub pkce_verifier: &'a str,
    pub redirect_uri: &'a str,
}

impl TokenExchange<'_> {
    /// トークンエンドポイントへ送る `application/x-www-form-urlencoded` の form パラメータ。
    pub(crate) fn to_form(&self) -> Vec<(&'static str, String)> {
        vec![
            ("client_id", self.client_id.to_string()),
            ("client_secret", self.client_secret.to_string()),
            ("code", self.code.to_string()),
            ("code_verifier", self.pkce_verifier.to_string()),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", self.redirect_uri.to_string()),
        ]
    }
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

    #[test]
    fn deserializes_token_response_with_refresh_token() -> ::anyhow::Result<()> {
        let json = r#"{"access_token":"at","expires_in":3599,"refresh_token":"rt","scope":"openid email","token_type":"Bearer","id_token":"idt"}"#;
        let response: TokenResponse = ::serde_json::from_str(json)?;
        assert_eq!(response.id_token, "idt");
        assert_eq!(response.refresh_token.as_deref(), Some("rt"));
        Ok(())
    }

    #[test]
    fn deserializes_token_response_without_refresh_token() -> ::anyhow::Result<()> {
        let json = r#"{"id_token":"idt","token_type":"Bearer"}"#;
        let response: TokenResponse = ::serde_json::from_str(json)?;
        assert_eq!(response.id_token, "idt");
        assert_eq!(response.refresh_token, None);
        Ok(())
    }

    #[test]
    fn token_exchange_to_form_has_required_pairs() {
        let form = TokenExchange {
            client_id: "cid",
            client_secret: "csecret",
            code: "the-code",
            pkce_verifier: "the-verifier",
            redirect_uri: "http://127.0.0.1/cb",
        }
        .to_form();
        assert!(form.contains(&("client_id", "cid".to_string())));
        assert!(form.contains(&("client_secret", "csecret".to_string())));
        assert!(form.contains(&("code", "the-code".to_string())));
        assert!(form.contains(&("code_verifier", "the-verifier".to_string())));
        assert!(form.contains(&("grant_type", "authorization_code".to_string())));
        assert!(form.contains(&("redirect_uri", "http://127.0.0.1/cb".to_string())));
    }
}
