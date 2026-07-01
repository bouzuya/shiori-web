// login が消費するまで bin ビルドでは未使用。消費側 (次の単位) を追加したら外す。
#![allow(dead_code)]

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
}
