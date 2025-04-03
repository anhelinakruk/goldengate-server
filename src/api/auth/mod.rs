use axum::extract::State;
use axum::http::HeaderValue;
use axum::routing::{get, post};
use axum::{http, Json, Router};
use hyper::HeaderMap;
use jsonwebtoken::TokenData;
use jsonwebtoken::{decode, encode, Header, Validation};
use models::{
    AuthBody, AuthError, Claims, GenerateNonceResponse, Keys, VerifySiweAndCreateUserRequest,
};
use siwe::{generate_nonce, Message};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use surrealdb::sql::Thing;

use crate::AppState;

use super::AppError;

pub mod models;

pub fn router(app_state: &AppState) -> Router {
    Router::new()
        .route("/", get(get_nonce))
        .route("/", post(verify_siwe_and_create_user))
        .with_state(app_state.clone())
}

async fn get_nonce(State(state): State<AppState>) -> Result<Json<GenerateNonceResponse>, AppError> {
    println!("Generating nonce");
    let nonce = generate_nonce();
    let result = save_nonce(nonce.clone(), State(state)).await;

    match result {
        Ok(_) => Ok(Json(GenerateNonceResponse { message: nonce })),
        Err(e) => Err(e),
    }
}

pub async fn save_nonce(value: String, state: State<AppState>) -> Result<(), AppError> {
    println!("Saving nonce");
    let _response = state.database
      .query("
        DELETE nonce WHERE exp < time::now() RETURN BEFORE;
        CREATE ONLY nonce SET value = type::string($value), exp = time::now() + 5m, iat = time::now();
      ")
        .bind(("value", value))
        .await?;

    println!("Nonce saved");

    Ok(())
}

pub async fn verify_siwe_and_create_user(
    State(state): State<AppState>,
    Json(payload): Json<VerifySiweAndCreateUserRequest>,
) -> Result<(HeaderMap, Json<AuthBody>), AppError> {
    println!("payload: {}", payload.message);

    let siwe_message = Message::from_str(&payload.message)?;

    let signature: [u8; 65] =
        prefix_hex::decode(&payload.signature).expect("Failed to decode signature");

    siwe_message
        .verify(&signature, &siwe::VerificationOpts::default())
        .await?;

    println!("SIWE message verified");

    let user_id = create_user(State(state.clone()), payload.address.clone()).await?;

    println!("User created");

    let token_str = generate_jwt(user_id.to_string(), State(state.clone())).await?;

    println!("Token generated");

    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::SET_COOKIE,
        HeaderValue::from_str(format!("{}={}", "token", token_str).as_str())?,
    );

    println!("Headers set");

    Ok((
        headers,
        Json(AuthBody {
            access_token: token_str,
        }),
    ))
}

// USER
pub async fn create_user(
    State(state): State<AppState>,
    address: String,
) -> Result<String, AppError> {
    let db = state.database.clone();

    println!("User address: {}", address);

    let user_id = match db
        .query("SELECT id FROM user WHERE address = type::string($address)")
        .bind(("address", address.clone()))
        .await?
        .take::<Option<Thing>>(0)?
    {
        Some(existing_user) => {
            println!("User found, returning user id");
            existing_user.to_string()
        }
        None => {
            println!("User not found, creating user");
            let user_id = db
                .query("CREATE ONLY user SET address = type::string($address) RETURN VALUE id")
                .bind(("address", address.clone()))
                .await?
                .take::<Option<Thing>>(0)?
                .ok_or(AppError(anyhow::anyhow!("Failed to create a user")))?;

            user_id.to_string()
        }
    };

    println!("User id: {}", user_id);
    Ok(user_id)
}

// JWT
pub async fn generate_jwt(id: String, app_state: State<AppState>) -> Result<String, AuthError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let user_claims = Claims {
        sub: id,
        exp: (now + 3600) as usize,
    };

    let secret = app_state.jwt_secret.clone();
    let keys = Keys::new(secret.as_bytes());

    let token_str = encode(&Header::default(), &user_claims, &keys.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    Ok(token_str)
}

async fn verify_jwt(token: String, app_state: &AppState) -> Result<TokenData<Claims>, AuthError> {
    let secret = app_state.jwt_secret.clone();
    let keys = Keys::new(secret.as_bytes());
    let token_data = decode::<Claims>(&token, &keys.decoding, &Validation::default())
        .map_err(|_| AuthError::InvalidToken)?;

    Ok(token_data)
}
