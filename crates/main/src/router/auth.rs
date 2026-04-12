mod callback;
mod signin;
mod signout;
mod signup;

use axum::Router;

use crate::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .merge(callback::router())
        .merge(signin::router())
        .merge(signout::router())
        .merge(signup::router())
}
