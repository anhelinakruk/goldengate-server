use axum::{extract::State, routing::get, Router};

use crate::AppState;

pub fn router(app_state: &AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .with_state(app_state.clone())
}

pub async fn root(State(_state): State<AppState>) -> String {
    "Hello, World public!".to_string()
}
