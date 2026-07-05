fn main() {
    println!("cargo:rerun-if-env-changed=SHIORI_EXPORT_URL");
    println!("cargo:rerun-if-env-changed=SHIORI_OIDC_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=SHIORI_OIDC_CLIENT_SECRET");
}
