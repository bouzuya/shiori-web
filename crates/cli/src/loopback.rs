/// loopback コールバックで受け取った認可コードと state。
pub(crate) struct CallbackParams {
    pub code: String,
    pub state: String,
}

/// HTTP リクエストの request-line (例: `GET /callback?code=X&state=Y HTTP/1.1`) から
/// 認可コードと state を取り出す。
///
/// Google が `error` パラメータ (例: `access_denied`) を返した場合は失敗として扱う。
pub(crate) fn parse_callback_request_line(request_line: &str) -> ::anyhow::Result<CallbackParams> {
    // request-line = METHOD SP request-target SP HTTP-version
    let target = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| ::anyhow::anyhow!("malformed request line"))?;
    // request-target は origin-form (path?query) なので base を付けてパースする。
    let url = ::url::Url::parse("http://127.0.0.1/")?.join(target)?;

    let mut code = None;
    let mut state = None;
    let mut error = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            _ => {}
        }
    }

    if let Some(error) = error {
        ::anyhow::bail!("authorization request failed: {error}");
    }
    let code = code.ok_or_else(|| ::anyhow::anyhow!("missing `code` in callback"))?;
    let state = state.ok_or_else(|| ::anyhow::anyhow!("missing `state` in callback"))?;
    Ok(CallbackParams { code, state })
}

/// 事前に bind した loopback リスナで1接続を待ち受け、コールバックの request-line から
/// `CallbackParams` を取り出す。ブラウザには「閉じてよい」旨の簡単な HTML を返す。
///
/// リスナを引数で受け取るのは、ポート確定 (redirect_uri 構築) と bind を呼び出し側に委ね、
/// テストでは port 0 で bind して結合テストできるようにするため。
pub(crate) async fn receive_callback(
    listener: ::tokio::net::TcpListener,
) -> ::anyhow::Result<CallbackParams> {
    let (mut stream, _peer) = listener.accept().await?;
    let (read_half, mut write_half) = stream.split();

    let mut reader = ::tokio::io::BufReader::new(read_half);
    let mut request_line = String::new();
    ::tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut request_line).await?;
    let params = parse_callback_request_line(request_line.trim_end());

    let body = match &params {
        Ok(_) => {
            "<!doctype html><meta charset=\"utf-8\"><p>ログインが完了しました。このタブは閉じてかまいません。</p>"
        }
        Err(_) => {
            "<!doctype html><meta charset=\"utf-8\"><p>ログインに失敗しました。ターミナルの出力を確認してください。</p>"
        }
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    ::tokio::io::AsyncWriteExt::write_all(&mut write_half, response.as_bytes()).await?;
    ::tokio::io::AsyncWriteExt::flush(&mut write_half).await?;

    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_code_and_state() -> ::anyhow::Result<()> {
        let params = parse_callback_request_line(
            "GET /callback?code=auth-code-123&state=state-xyz HTTP/1.1",
        )?;
        assert_eq!(params.code, "auth-code-123");
        assert_eq!(params.state, "state-xyz");
        Ok(())
    }

    #[test]
    fn decodes_percent_encoded_values() -> ::anyhow::Result<()> {
        let params =
            parse_callback_request_line("GET /callback?code=a%2Fb%3Dc&state=s%20t HTTP/1.1")?;
        assert_eq!(params.code, "a/b=c");
        assert_eq!(params.state, "s t");
        Ok(())
    }

    #[test]
    fn errors_when_code_is_missing() {
        assert!(parse_callback_request_line("GET /callback?state=s HTTP/1.1").is_err());
    }

    #[test]
    fn errors_when_state_is_missing() {
        assert!(parse_callback_request_line("GET /callback?code=c HTTP/1.1").is_err());
    }

    #[test]
    fn surfaces_authorization_error_param() {
        assert!(
            parse_callback_request_line("GET /callback?error=access_denied&state=s HTTP/1.1")
                .is_err()
        );
    }

    #[test]
    fn errors_on_malformed_request_line() {
        assert!(parse_callback_request_line("GARBAGE").is_err());
    }

    #[::tokio::test]
    async fn receive_callback_returns_params_and_responds_ok() -> ::anyhow::Result<()> {
        let listener = ::tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        let server = ::tokio::spawn(receive_callback(listener));

        let mut client = ::tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
        ::tokio::io::AsyncWriteExt::write_all(
            &mut client,
            b"GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: localhost\r\n\r\n",
        )
        .await?;
        let mut response = Vec::new();
        ::tokio::io::AsyncReadExt::read_to_end(&mut client, &mut response).await?;

        let params = server.await??;
        assert_eq!(params.code, "abc");
        assert_eq!(params.state, "xyz");
        assert!(String::from_utf8_lossy(&response).contains("200 OK"));
        Ok(())
    }
}
