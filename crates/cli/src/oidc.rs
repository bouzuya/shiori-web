/// 認可コードフロー (loopback + PKCE) の認可リクエスト。
///
/// `authorization_url` をブラウザで開き、コールバックで受け取った認可コードを
/// `code_verifier` と共にトークンエンドポイントへ送って交換する。
/// `csrf_state` はコールバックの `state` と突き合わせて改ざんを検査する。
pub(crate) struct AuthorizationRequest {
    pub authorization_url: String,
    pub code_verifier: String,
    pub csrf_state: String,
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
        code_verifier: pkce_verifier.secret().to_string(),
        csrf_state: csrf_state.secret().to_string(),
    })
}

/// トークンエンドポイントの応答 (必要なフィールドのみ。未知フィールドは無視)。
#[derive(Clone, Debug, Eq, PartialEq, ::serde::Deserialize)]
pub(crate) struct TokenResponse {
    // 5d の export が refresh 後に Bearer として使うまで、login では未読。
    #[allow(dead_code)]
    pub id_token: String,
    pub refresh_token: Option<String>,
}

/// トークンエンドポイントへ送る `application/x-www-form-urlencoded` のリクエスト表現。
///
/// Google のデスクトップ型クライアントはトークン交換に client_secret を要求するが、
/// これは「秘密として扱わない」見せかけの secret で、実際の保護は PKCE の code_verifier が担う。
#[derive(::serde::Serialize)]
pub(crate) struct TokenExchange<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub code: &'a str,
    pub code_verifier: &'a str,
    pub grant_type: &'a str,
    pub redirect_uri: &'a str,
}

/// 認可コードをトークンエンドポイントで交換し、`TokenResponse` を得る。
pub(crate) async fn exchange_code(
    token_endpoint: &str,
    exchange: &TokenExchange<'_>,
) -> ::anyhow::Result<TokenResponse> {
    let response = ::reqwest::Client::new()
        .post(token_endpoint)
        .form(exchange)
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        ::anyhow::bail!("token endpoint returned {status}: {body}");
    }
    Ok(::serde_json::from_str(&body)?)
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
        assert!(!request.code_verifier.is_empty());
        Ok(())
    }

    #[test]
    fn generates_distinct_code_verifier_and_state_per_call() -> ::anyhow::Result<()> {
        let a =
            build_authorization_request("https://e.example/auth", "c", "http://127.0.0.1:1/cb")?;
        let b =
            build_authorization_request("https://e.example/auth", "c", "http://127.0.0.1:1/cb")?;
        assert_ne!(a.code_verifier, b.code_verifier);
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
    fn token_exchange_serializes_to_expected_form_pairs() -> ::anyhow::Result<()> {
        let encoded = ::serde_urlencoded::to_string(TokenExchange {
            client_id: "cid",
            client_secret: "csecret",
            code: "the-code",
            code_verifier: "the-verifier",
            grant_type: "authorization_code",
            redirect_uri: "http://127.0.0.1/cb",
        })?;
        let params: ::std::collections::HashMap<String, String> =
            ::url::form_urlencoded::parse(encoded.as_bytes())
                .into_owned()
                .collect();
        assert_eq!(params.get("client_id").map(String::as_str), Some("cid"));
        assert_eq!(
            params.get("client_secret").map(String::as_str),
            Some("csecret")
        );
        assert_eq!(params.get("code").map(String::as_str), Some("the-code"));
        assert_eq!(
            params.get("code_verifier").map(String::as_str),
            Some("the-verifier")
        );
        assert_eq!(
            params.get("grant_type").map(String::as_str),
            Some("authorization_code")
        );
        assert_eq!(
            params.get("redirect_uri").map(String::as_str),
            Some("http://127.0.0.1/cb")
        );
        Ok(())
    }

    /// 1接続を受けて (リクエストを読み切ってから) 固定の HTTP 応答を返すモック。
    async fn spawn_token_endpoint(
        status_line: &'static str,
        body: String,
    ) -> ::anyhow::Result<(String, ::tokio::task::JoinHandle<::anyhow::Result<()>>)> {
        let listener = ::tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let url = format!("http://{}/token", listener.local_addr()?);
        let handle = ::tokio::spawn(async move {
            let (mut stream, _peer) = listener.accept().await?;
            let (read_half, mut write_half) = stream.split();
            let mut reader = ::tokio::io::BufReader::new(read_half);

            let mut content_length: usize = 0;
            loop {
                let mut line = String::new();
                let read = ::tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await?;
                if read == 0 || line.trim_end().is_empty() {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut request_body = vec![0u8; content_length];
            ::tokio::io::AsyncReadExt::read_exact(&mut reader, &mut request_body).await?;

            let response = format!(
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            ::tokio::io::AsyncWriteExt::write_all(&mut write_half, response.as_bytes()).await?;
            ::tokio::io::AsyncWriteExt::flush(&mut write_half).await?;
            Ok(())
        });
        Ok((url, handle))
    }

    fn sample_exchange() -> TokenExchange<'static> {
        TokenExchange {
            client_id: "cid",
            client_secret: "sec",
            code: "the-code",
            code_verifier: "the-verifier",
            grant_type: "authorization_code",
            redirect_uri: "http://127.0.0.1/cb",
        }
    }

    #[::tokio::test]
    async fn exchange_code_posts_form_and_parses_token_response() -> ::anyhow::Result<()> {
        let (url, server) = spawn_token_endpoint(
            "200 OK",
            r#"{"id_token":"idt","refresh_token":"rt","token_type":"Bearer"}"#.to_string(),
        )
        .await?;
        let token = exchange_code(&url, &sample_exchange()).await?;
        server.await??;
        assert_eq!(token.id_token, "idt");
        assert_eq!(token.refresh_token.as_deref(), Some("rt"));
        Ok(())
    }

    #[::tokio::test]
    async fn exchange_code_errors_on_non_success_status() -> ::anyhow::Result<()> {
        let (url, server) = spawn_token_endpoint(
            "400 Bad Request",
            r#"{"error":"invalid_grant"}"#.to_string(),
        )
        .await?;
        let result = exchange_code(&url, &sample_exchange()).await;
        server.await??;
        assert!(result.is_err());
        Ok(())
    }
}
