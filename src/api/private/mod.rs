use crate::api::public::models::Offer;
use crate::AppState;
use alloy::hex::FromHex;
use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, U256};
use alloy::providers::Provider;
use alloy::rpc::types::TransactionRequest;
use alloy::signers::k256::ecdsa::SigningKey;
use alloy::signers::local::PrivateKeySigner;
use alloy::{primitives::FixedBytes, providers::ProviderBuilder, sol};
use axum::extract::Path;
use axum::routing::delete;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use std::ops::Div;

use models::{
    ConfirmDepositRequest, CreateOfferRequest, CreateTransactionRequest, GetAggregatedFeeRequest,
    GetAggregatedFeeResponse, GetBalanceResponse, WithdrawRequest,
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
        .route("/withdraw", post(withdraw))
        .route("/fee", post(get_aggregated_fee))
        .route("/balance", get(get_balance))
        .route("/user/offers", get(get_user_offers))
        .route("/user/offers/{id}", delete(delete_offer))
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
    _claims: Claims,
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

sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);

    interface IERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
}

pub async fn confirm_deposit(
    State(state): State<AppState>,
    _claims: Claims,
    Json(payload): Json<ConfirmDepositRequest>,
) -> Result<(), AppError> {
    println!("Confirming deposit");
    println!("payload: {:?}", payload);

    let rpc_url = state.alchemy_rpc_url.parse()?;
    let provider = ProviderBuilder::new().on_http(rpc_url);

    let confirming_blocks = state.confirming_blocks;
    let tx_hash = FixedBytes::from_str(&payload.tx_hash)?;

    tokio::spawn(async move {
        for retries in 0..15 {
            println!("Retrying attempt {}", retries + 1);
            println!("Tx hash: {:?}", tx_hash);

            let receipt = provider.get_transaction_receipt(tx_hash).await;

            println!("Receipt: {:?}", receipt);

            match receipt {
                Ok(receipt) => {
                    if let Some(receipt) = receipt {
                        println!("Receipt found");
                        println!("Receipt logs: {:?}", receipt.logs());

                        let block_number = receipt.block_number.ok_or_else(|| {
                            AppError::from(anyhow::anyhow!("No block number found"))
                        })?;
                        let current_block = provider.get_block_number().await?;

                        let maybe_log = receipt.decoded_log::<Transfer>();

                        let Some(increment_log) = maybe_log else {
                            return Err(AppError::from(anyhow::anyhow!("Increment not emitted")));
                        };

                        let Transfer { from, to, value } = increment_log.data;
                        println!("Incremented value: {from} -> {to} = {value}");

                        let decrement_log = receipt.decoded_log::<Transfer>();

                        let Some(decrement_log) = decrement_log else {
                            return Err(AppError::from(anyhow::anyhow!("Decrement not emitted")));
                        };

                        let Transfer { from, to, value } = decrement_log.data;
                        println!("Decremented value: {from} -> {to} = {value}");
                        println!("Confirming blocks: {}", confirming_blocks);
                        println!("Transaction Block: {}", block_number);
                        println!("Current Block: {}", current_block);

                        if current_block - block_number >= confirming_blocks {
                            println!("Block confirmed");
                            println!("Receipt from: {:?}", from);
                            println!("Amount: {:?}", value.div(U256::from(10u128.pow(12))));

                            tokio::spawn(async move {
                                let _ = state
                                    .database
                                    .query(
                                        "UPDATE ONLY user SET balance = balance + type::number($amount) WHERE address = type::string($address) RETURN id, balance;",
                                    )
                                    .bind(("address", from.to_string().to_lowercase()))
                                    .bind(("amount", value.div(U256::from(10u128.pow(12))).to_string()))
                                    .await;

                                println!("Balance updated for address: {}", from);
                            });
                            println!("Deposit confirmed!");
                            return Ok(());
                        }
                    }
                }
                Err(err) => {
                    println!("Error getting transaction receipt: {}", err);
                }
            }
            tokio::time::sleep(Duration::from_secs(20)).await;
        }

        println!("Transaction confirmation failed");
        Err(AppError::from(anyhow::anyhow!(
            "Transaction confirmation failed after 15 attempts"
        )))
    });

    Ok(())
}

pub async fn withdraw(
    State(state): State<AppState>,
    claims: Claims,
    Json(payload): Json<WithdrawRequest>,
) -> Result<(), AppError> {
    println!("Withdrawing");
    println!("payload: {:?}", payload);

    // Update the balance to prevent double spending
    let mut user_balance = state
        .database
        .query("SELECT VALUE balance FROM user WHERE id = type::thing($id)")
        .bind(("id", claims.sub.clone()))
        .await?;

    let user_balance = user_balance
        .take::<Option<i128>>(0)
        .map_err(AppError::from)?
        .ok_or(AppError::from(anyhow::anyhow!("Balance not found")))?;

    println!("User balance: {:?}", user_balance);

    if user_balance < payload.amount {
        return Err(AppError::from(anyhow::anyhow!("Insufficient balance")));
    }

    let mut update_balance = state
        .database
        .query(
            "UPDATE ONLY user SET balance = balance - type::number($amount) WHERE id = type::thing($id) RETURN VALUE balance;",
        )
        .bind(("id", claims.sub.clone()))
        .bind(("amount", payload.amount))
        .await?;

    println!("Update balance: {:?}", update_balance);

    let updated_balance = update_balance
        .take::<Option<i128>>(0)
        .map_err(AppError::from)?
        .ok_or(AppError::from(anyhow::anyhow!("Balance not found")))?;

    println!("Updated balance: {:?}", updated_balance);

    // Sign the transaction
    let key_bytes: [u8; 32] = <[u8; 32]>::from_hex(&state.private_key)
        .map_err(|_| AppError::from(anyhow::anyhow!("Invalid private key")))?;

    let private_key = SigningKey::from_slice(&key_bytes).unwrap();
    let signer = PrivateKeySigner::from_signing_key(private_key);

    println!("Signer: {:?}", signer);

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .on_http(state.alchemy_rpc_url.parse()?);

    println!("Provider: {:?}", provider);

    let token_address = Address::from_str(&state.token_address)?;
    let to_address = Address::from_str(&payload.address)?;
    let amount = U256::from(payload.amount * 10i128.pow(12));
    let call = IERC20::transferCall {
        to: to_address,
        amount,
    };

    println!("Token address: {:?}", token_address);
    println!("To address: {:?}", to_address);
    println!("Amount: {:?}", amount);

    let tx = TransactionRequest::default()
        .with_to(token_address)
        .with_value(U256::from(0))
        .with_call(&call);

    let gas_estimate = provider.estimate_gas(tx.clone()).await?;

    println!("Transaction: {:?}", tx);
    println!("Gas estimate: {:?}", gas_estimate);

    let tx_hash = provider
        .send_transaction(tx.with_gas_limit(gas_estimate))
        .await?
        .watch()
        .await?;

    println!("Transaction sent: {:?}", tx_hash);

    tokio::spawn(async move {
        for retries in 0..15 {
            println!("Retrying attempt {}", retries + 1);
            println!("Tx hash: {:?}", tx_hash);

            let receipt = provider.get_transaction_receipt(tx_hash).await;
            let confirming_blocks = state.confirming_blocks;

            println!("Receipt: {:?}", receipt);

            match receipt {
                Ok(receipt) => {
                    if let Some(receipt) = receipt {
                        println!("Receipt found");
                        println!("Receipt logs: {:?}", receipt.logs());

                        let block_number = receipt.block_number.ok_or_else(|| {
                            AppError::from(anyhow::anyhow!("No block number found"))
                        })?;
                        let current_block = provider.get_block_number().await?;

                        let maybe_log = receipt.decoded_log::<Transfer>();

                        let Some(increment_log) = maybe_log else {
                            return Err(AppError::from(anyhow::anyhow!("Increment not emitted")));
                        };

                        let Transfer { from, to, value } = increment_log.data;
                        println!("Incremented value: {from} -> {to} = {value}");

                        let decrement_log = receipt.decoded_log::<Transfer>();

                        let Some(decrement_log) = decrement_log else {
                            return Err(AppError::from(anyhow::anyhow!("Decrement not emitted")));
                        };

                        let Transfer { from, to, value } = decrement_log.data;
                        println!("Decremented value: {from} -> {to} = {value}");
                        println!("Confirming blocks: {}", confirming_blocks);
                        println!("Transaction Block: {}", block_number);
                        println!("Current Block: {}", current_block);

                        if current_block - block_number >= confirming_blocks {
                            println!("Block confirmed");
                            println!("Receipt from: {:?}", from);
                            println!("Amount: {:?}", value.div(U256::from(10u128.pow(12))));

                            let _ = state
                                    .database
                                    .query(
                                        "UPDATE ONLY user SET balance = balance + type::number($amount) WHERE id = type::thing($id) RETURN balance;",
                                    )
                                    .bind(("id", from.to_string().to_lowercase()))
                                    .bind(("amount", value.div(U256::from(10u128.pow(12))).to_string()))
                                    .await;

                            println!("Balance updated for address: {}", from);
                            return Ok(());
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
            "Transaction confirmation failed after 15 attempts"
        )))
    });

    Ok(())
}

pub async fn get_balance(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<GetBalanceResponse>, AppError> {
    println!("Getting balance");
    let mut balance = state
        .database
        .query("
            LET $balance = MATH::SUM(SELECT VALUE balance FROM user WHERE walletAddress = $walletAddress);

            LET $open_offers = MATH::SUM(SELECT VALUE amount + fee FROM offers WHERE userId.walletAddress = $walletAddress AND status != 'closed');

            LET $closed_offers = MATH::SUM(SELECT VALUE amount + takerFee + makerFee FROM transactions WHERE offerId.userId.walletAddress = $walletAddress AND status = 'successful' AND offerId.status = 'closed');

            RETURN $balance - $open_offers - $closed_offers;
        ")
        .bind(("id", claims.sub.clone()))
        .await?;

    println!("Response: {:?}", balance);

    let balance = balance
        .take::<Option<i128>>(3)
        .map_err(AppError::from)?
        .ok_or(AppError::from(anyhow::anyhow!("Balance not found")))?;

    println!("Balance: {:?}", balance);

    Ok(Json(GetBalanceResponse { balance }))
}

pub async fn get_user_offers(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<Offer>>, AppError> {
    println!("Getting user offers");

    let mut offers = state
        .database
        .query("
            SELECT id , (amount - MATH::SUM(SELECT VALUE amount+takerFee
            FROM transactions 
            WHERE offerId = $parent.id AND status != type::string('rejected'))) as amount, 
            cryptoType, currency, pricePerUnit, value, offerType, revTag, fee, status
            FROM offers 
            WHERE status != type::string('closed') AND userId = type::thing($userId) AND amount - MATH::SUM(SELECT VALUE amount+takerFee
            FROM transactions 
            WHERE offerId = $parent.id AND status != type::string('rejected')) > 0;
        ")
        .bind(("userId", claims.sub.clone()))
        .await?;

    println!("Response: {:?}", offers);

    let offers = offers.take::<Vec<Offer>>(0).map_err(AppError::from)?;

    println!("Offers: {:?}", offers);

    Ok(Json(offers))
}

pub async fn delete_offer(
    State(state): State<AppState>,
    _claims: Claims,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    println!("Deleting offer with id: {}", id);

    let _ = state
        .database
        .query("
            IF COUNT(SELECT * FROM transactions WHERE status='pending' AND offerId = type::thing($id)) > 0 THEN {
					UPDATE offers SET status = type::string('stopped') WHERE id = type::thing($id);
			} ELSE {
					UPDATE offers SET status = type::string('closed') WHERE id = type::thing($id);
			} END;
        ")
        .bind(("id", id))
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
