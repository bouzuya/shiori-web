# shiori-web

A bookmark management app by bouzuya.

- <https://addons.mozilla.org/en-US/firefox/addon/side-view/>

## CLI quick start

This repository uses `_env` as the local environment template.
Create `.env` first.

```bash
cp _env .env
```

### 1. Start server

Set these values and run the web server.

- `OIDC_CLI_CLIENT_ID`: Google OAuth client ID for CLI audience verification on server side
- `OIDC_CLIENT_ID`
- `OIDC_CLIENT_SECRET`
- `OIDC_ISSUER_URL`
- `OIDC_REDIRECT_URI`
- `COOKIE_SIGNING_SECRET`
- `PROJECT_ID`
- `DATABASE_ID`

Then start server.

```bash
cargo run --bin main -- serve
```

### 2. Login from CLI

The CLI needs no build-time settings. Pass the server URL to `login`;
the CLI fetches the OIDC client configuration from `{SERVER_URL}/cli/config`
and discovers the provider endpoints from the issuer.

```bash
cargo run -p cli -- login http://127.0.0.1:3000
```

With base path (`BASE_PATH=/base`), include it in the server URL.

```bash
cargo run -p cli -- login http://127.0.0.1:3000/base
```

`login` saves the server URL to `$XDG_CONFIG_HOME/shiori/config.json`
and the refresh token to `$XDG_STATE_HOME/shiori/token.json`.

### 3. Export bookmarks as NDJSON

Quick connectivity check after starting server:

```bash
curl -i http://127.0.0.1:3000/export | head -n 1
# expected: HTTP/1.1 401 Unauthorized
```

`export` uses `{SERVER_URL}/export` for the server URL saved by `login`.

```bash
cargo run -p cli -- export
```

Pipe examples.

```bash
cargo run -p cli -- export | jq
cargo run -p cli -- export | fzf
```

## Troubleshooting

- `export request failed with 401 Unauthorized` or `403 Forbidden`: run `cargo run -p cli -- login <SERVER_URL>` again, then confirm the server's `OIDC_CLI_CLIENT_ID` / `OIDC_CLI_CLIENT_SECRET` point to the CLI OAuth client.
- `failed to call export endpoint ... is the server running and URL correct?`: start server with `cargo run --bin main -- serve`. If the server URL (or base path) changed, run `login <SERVER_URL>` again.
- `no server configured. run \`shiori login <SERVER_URL>\` first`: run `login` once to save the server URL.
