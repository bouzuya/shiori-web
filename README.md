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

### 2. Build and login from CLI

The CLI reads these at build time via `option_env!`.

- `SHIORI_OIDC_CLIENT_ID`
- `SHIORI_OIDC_CLIENT_SECRET`
- `SHIORI_EXPORT_URL`

If you change any value, rebuild the CLI binary before `login` / `export`.

```bash
cargo clean -p cli
```

Login once.

```bash
cargo run -p cli -- login
```

### 3. Export bookmarks as NDJSON

#### No base path (`BASE_PATH=`)

Quick connectivity check after starting server:

```bash
curl -i http://127.0.0.1:3000/export | head -n 1
# expected: HTTP/1.1 401 Unauthorized
```

```bash
cargo run -p cli -- export
```

#### With base path (`BASE_PATH=/base`)

Set `SHIORI_EXPORT_URL` at build time (see step 2), or use `--url` for a one-off run.

```bash
cargo run -p cli -- export --url http://127.0.0.1:3000/base/export
```

Pipe examples.

```bash
cargo run -p cli -- export | jq
cargo run -p cli -- export | fzf
```

## Troubleshooting

- `export request failed with 401 Unauthorized` or `403 Forbidden`: run `cargo run -p cli -- login` again, then confirm `OIDC_CLI_CLIENT_ID` (server) and `SHIORI_OIDC_CLIENT_ID` (CLI build-time) point to the same OAuth client.
- `failed to call export endpoint ... is the server running and URL correct?`: start server with `cargo run --bin main -- serve`, then check `SHIORI_EXPORT_URL` (CLI build-time) or `--url` if you use a base path.
- `this binary was built without SHIORI_OIDC_CLIENT_ID`: set `SHIORI_OIDC_CLIENT_ID` and `SHIORI_OIDC_CLIENT_SECRET` in `.env`, then run `cargo clean -p cli` and rebuild.
