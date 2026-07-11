/// 1接続を受けて固定の HTTP 応答を返すモック。
pub(crate) async fn spawn_json_server(
    status_line: &'static str,
    body: String,
) -> ::anyhow::Result<(String, ::tokio::task::JoinHandle<::anyhow::Result<()>>)> {
    let listener = ::tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!("http://{}", listener.local_addr()?);
    let handle = ::tokio::spawn(async move {
        let (mut stream, _peer) = listener.accept().await?;
        let (read_half, mut write_half) = stream.split();
        let mut reader = ::tokio::io::BufReader::new(read_half);

        loop {
            let mut line = String::new();
            let read = ::tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await?;
            if read == 0 || line.trim_end().is_empty() {
                break;
            }
        }

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
