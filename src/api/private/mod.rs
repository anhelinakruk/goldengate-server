use axum::{extract::State, routing::get, Router};

use crate::AppState;

use super::auth::models::Claims;

pub fn router(app_state: &AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .with_state(app_state.clone())
}

pub async fn root(State(_state): State<AppState>, claims: Claims) -> String {
    format!("Hello, World private! - {}", claims.sub)
}
