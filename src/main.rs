use args::Args;
use axum::{routing::get, Router};
use clap::Parser;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::{Error, Surreal};
use thiserror::Error;

pub mod api;
mod args;

#[derive(Debug, Clone)]
pub struct AppState {
    pub database: Surreal<Client>,
    pub jwt_secret: String,
    pub alchemy_rpc_url: String,
    pub confirming_blocks: u64,
}

impl AppState {
    pub async fn new(
        address: &str,
        username: &str,
        password: &str,
        namespace: &str,
        database: &str,
        jwt_secret: &str,
        alchemy_rpc_url: &str,
        confirming_blocks: u64,
    ) -> Result<Self, Error> {
        let client = Surreal::new::<Ws>(address).await?;
        client.signin(Root { username, password }).await?;
        client.use_ns(namespace).use_db(database).await?;

        Ok(AppState {
            database: client,
            jwt_secret: jwt_secret.to_string(),
            alchemy_rpc_url: alchemy_rpc_url.to_string(),
            confirming_blocks,
        })
    }
}

#[derive(Debug, Error)]
enum ServerError {
    #[error(transparent)]
    Surrealdb(#[from] surrealdb::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let args = Args::parse();

    let app_state = AppState::new(
        &args.surrealdb_address,
        &args.surrealdb_username,
        &args.surrealdb_password,
        &args.surrealdb_namespace,
        &args.surrealdb_database,
        &args.jwt_secret,
        &args.alchemy_rpc_url,
        args.confirming_blocks,
    )
    .await?;

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .nest("/auth", api::auth::router(&app_state))
        .nest("/private", api::private::router(&app_state))
        .nest("/public", api::public::router(&app_state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}
