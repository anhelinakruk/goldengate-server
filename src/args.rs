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
}
