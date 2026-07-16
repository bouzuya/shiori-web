pub(crate) mod export;
pub(crate) mod login;

#[derive(::clap::Subcommand)]
pub(crate) enum Subcommand {
    /// Export bookmarks from the logged-in server as NDJSON to stdout.
    Export(self::export::ExportArgs),
    /// Log in to the given server via OIDC (loopback + PKCE) and save the token and target locally.
    Login(self::login::LoginArgs),
}

impl Subcommand {
    pub(super) async fn execute(self) -> ::anyhow::Result<()> {
        match self {
            Subcommand::Export(args) => args.execute().await,
            Subcommand::Login(args) => args.execute().await,
        }
    }
}
