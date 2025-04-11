use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long, env, default_value = "0.0.0.0:8000")]
    pub surrealdb_address: String,

    #[arg(long, env, default_value = "root")]
    pub surrealdb_username: String,

    #[arg(long, env, default_value = "root")]
    pub surrealdb_password: String,

    #[arg(long, env, default_value = "prod")]
    pub surrealdb_namespace: String,

    #[arg(long, env, default_value = "prod")]
    pub surrealdb_database: String,

    #[arg(long, env, default_value = "secret")]
    pub jwt_secret: String,

    #[arg(long, env)]
    pub alchemy_rpc_url: String,

    #[arg(long, env, default_value = "6")]
    pub confirming_blocks: u64,

    #[arg(long, env)]
    pub wallet_address: String,

    #[arg(long, env)]
    pub private_key: String,

    #[arg(long, env)]
    pub token_address: String,
}
