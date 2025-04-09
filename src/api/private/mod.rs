use crate::api::private::models::ConfirmDepositResponse;
use crate::AppState;
use alloy::providers::Provider;
use alloy::{primitives::FixedBytes, providers::ProviderBuilder};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use models::{
    ConfirmDepositRequest, CreateOfferRequest, CreateTransactionRequest, GetAggregatedFeeRequest,
    GetAggregatedFeeResponse,
};
use std::{str::FromStr, time::Duration};

use super::{auth::models::Claims, AppError};

pub mod models;

pub fn router(app_state: &AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/offers", post(create_offer))
        .route("/transactions", post(create_transaction))
        .route("/deposit", post(confirm_deposit))
        .route("/fee", post(get_aggregated_fee))
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
        UPDATE transactions SET status = type::string('rejected') WHERE expiresAt < time::now() AND status = type::string('pending');
        FOR $id IN (SELECT VALUE id FROM offers WHERE status == 'stopped') {
                IF COUNT(SELECT * FROM transactions WHERE status='pending' AND offerId = type::thing($id)) = 0 THEN {
                        UPDATE offers SET status = type::string('closed') WHERE id = type::thing($id);
                } END;
        };
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
            UPDATE transactions SET status = type::string('rejected') WHERE expiresAt < time::now() AND status = type::string('pending');
            FOR $id IN (SELECT VALUE id FROM offers WHERE status == 'stopped') {
                    IF COUNT(SELECT * FROM transactions WHERE status='pending' AND offerId = type::thing($id)) = 0 THEN {
                            UPDATE offers SET status = type::string('closed') WHERE id = type::thing($id);
                    } END;
            };
            CREATE transactions SET 
            offerId = type::thing($offerId), 
            amount = type::number($amount), 
            cryptoType = type::string($cryptoType), 
            pricePerUnit = type::number($pricePerUnit), 
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

pub async fn get_aggregated_fee(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<GetAggregatedFeeRequest>,
) -> Result<Json<GetAggregatedFeeResponse>, AppError> {
    println!("Getting aggregated fee");
    println!("payload: {:?}", payload);
    let mut response =state
        .database
        .query(
            "MATH::SUM(SELECT VALUE makerFee FROM transactions WHERE offerId = type::thing($offerId) AND status != type::string('rejected'))",
        )
        .bind(("offerId", payload.offer_id))
        .await?;

    println!("Fee aggregated");

    let result: Option<i128> = response.take(0).map_err(AppError::from)?;

    println!("Result: {:?}", result);

    if let Some(result) = result {
        Ok(Json(GetAggregatedFeeResponse { fee: result }))
    } else {
        Err(AppError::from(anyhow::anyhow!("No result found")))
    }
}

pub async fn confirm_deposit(
    State(state): State<AppState>,
    // claims: Claims,
    Json(payload): Json<ConfirmDepositRequest>,
) -> Result<Json<ConfirmDepositResponse>, AppError> {
    println!("Confirming deposit");
    println!("payload: {:?}", payload);

    let rpc_url = state.alchemy_rpc_url.parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);

    let confirming_blocks = state.confirming_blocks;

    let tx_hash = FixedBytes::from_str(&payload.tx_hash)?;

    for retries in 0..15 {
        println!("Retrying attempt {}", retries + 1);
        println!("Tx hash: {:?}", tx_hash);

        let receipt = provider.get_transaction_receipt(tx_hash).await;

        println!("Receipt: {:?}", receipt);

        match receipt {
            Ok(receipt) => {
                if let Some(receipt) = receipt {
                    println!("Receipt found");

                    let block_number = receipt
                        .block_number
                        .ok_or_else(|| AppError::from(anyhow::anyhow!("No block number found")))?;
                    let current_block = provider.get_block_number().await?;

                    println!("Confirming blocks: {}", confirming_blocks);
                    println!("Transaction Block: {}", block_number);
                    println!("Current Block: {}", current_block);

                    if current_block - block_number >= confirming_blocks {
                        println!("Block confirmed");
                        println!("Receipt from: {:?}", receipt.from);
                        println!("Receipt to: {:?}", payload.amount);

                        let mut respone = state
                            .database
                            .query(
                                "UPDATE user SET balance = balance + type::number($amount) WHERE address = type::string($address) RETURN id, balance;",
                            )
                            .bind(("address", receipt.from.to_string().to_lowercase()))
                            .bind(("amount", payload.amount))
                            .await?;

                        println!("Response: {:?}", respone);

                        let updated_balance: Vec<ConfirmDepositResponse> =
                            respone.take(0).map_err(AppError::from)?;

                        println!("Updated balance: {:?}", updated_balance);

                        return Ok(Json(updated_balance[0].clone()));
                    }
                }
            }
            Err(err) => {
                println!("Error getting transaction receipt: {}", err);
            }
        }

        tokio::time::sleep(Duration::from_secs(20)).await;
    }

    Err(AppError::from(anyhow::anyhow!(
        "Transaction confirmation failed"
    )))
}
