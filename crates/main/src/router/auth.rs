mod callback;
mod signin;
mod signout;
mod signup;

use crate::AppState;

pub(crate) fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(callback::router())
        .merge(signin::router())
        .merge(signout::router())
        .merge(signup::router())
}
