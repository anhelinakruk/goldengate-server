use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use models::{CreateOfferRequest, CreateTransactionRequest};

use crate::AppState;

use super::{auth::models::Claims, AppError};

pub mod models;

pub fn router(app_state: &AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/offers", post(create_offer))
        .route("/transactions", post(create_transaction))
        .with_state(app_state.clone())
}

pub async fn root(State(_state): State<AppState>, claims: Claims) -> String {
    format!("Hello, World private! - {}", claims.sub)
}

pub async fn create_offer(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<CreateOfferRequest>,
) -> Result<(), AppError> {
    println!("Creating offer");
    println!("payload: {:?}", payload);
    state
        .database
        .query(
            "
        CREATE offers SET 
			amount = type::number($amount),
			fee = type::number($fee),
			cryptoType = type::string($cryptoType),
			currency = type::string($currency), 
			pricePerUnit = type::number($pricePerUnit), 
			value = type::number($value),
			offerType = type::string($offerType), 
			revTag = type::string($revTag),
			userId = type::thing($userId),
			status = type::string('open');",
        )
        .bind(payload)
        .bind(("userId", claims.sub))
        .await?;

    Ok(())
}

pub async fn create_transaction(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<CreateTransactionRequest>,
) -> Result<(), AppError> {
    println!("Creating transaction");
    println!("payload: {:?}", payload);
    state
        .database
        .query(
            "
            CREATE transactions SET 
            offerId = type::thing($offerId), 
            amount = type::number($amount), 
            cryptoType = type::string($cryptoType), 
            price = type::number($price), 
            currency = type::string($currency), 
            takerFee = type::number($takerFee),
            makerFee = type::number($makerFee),
            value = type::number($value),
            expiresAt = time::now() + 5m, 
            status = type::string('pending'),
            randomTitle = type::string($randomTitle),
            userId = type::thing($userId);
        ",
        )
        .bind(payload)
        .bind(("userId", claims.sub))
        .await?;

    println!("Transaction created");

    Ok(())
}
