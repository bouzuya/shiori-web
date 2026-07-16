pub(crate) mod export;
pub(crate) mod login;

#[derive(::clap::Subcommand)]
pub(crate) enum Subcommand {
    /// Export bookmarks from the logged-in server as NDJSON to stdout.
    Export(ExportArgs),
    /// Log in to the given server via OIDC (loopback + PKCE) and save the token and target locally.
    Login(LoginArgs),
}

#[derive(::clap::Args)]
pub(crate) struct ExportArgs {
    /// Discard the local cache and refetch all bookmarks
    /// (required to reflect bookmarks deleted on the server).
    #[arg(long)]
    pub refresh: bool,
}

#[derive(::clap::Args)]
pub(crate) struct LoginArgs {
    #[arg(default_value_t = 9787, env = "SHIORI_LOOPBACK_PORT", long)]
    pub port: u16,
    /// The shiori server URL to connect to (e.g. https://shiori.example.com)
    pub server_url: String,
}

impl Subcommand {
    pub(super) async fn execute(self) -> ::anyhow::Result<()> {
        match self {
            Subcommand::Export(args) => {
                self::export::run(self::export::ExportConfig::resolve().await?, args.refresh).await
            }
            Subcommand::Login(args) => {
                self::login::run(
                    self::login::LoginConfig::fetch(&args.server_url, args.port).await?,
                )
                .await
            }
        }
    }
}
