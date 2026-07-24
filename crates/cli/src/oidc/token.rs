#[derive(::serde::Deserialize)]
pub(crate) struct RefreshTokenResponse {
    pub(crate) id_token: String,
}

#[derive(::serde::Serialize)]
pub(crate) struct TokenRefresh<'a> {
    pub(crate) client_id: &'a str,
    pub(crate) client_secret: &'a str,
    pub(crate) grant_type: &'a str,
    pub(crate) refresh_token: &'a str,
}
