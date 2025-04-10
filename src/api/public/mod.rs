use axum::{extract::State, routing::get, Json, Router};
use models::{DepositAddressResponse, Offer};

use crate::AppState;

use super::AppError;

pub mod models;

pub fn router(app_state: &AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/offers", get(get_offers))
        .route("/address", get(get_deposit_address))
        .with_state(app_state.clone())
}

pub async fn root(State(_state): State<AppState>) -> String {
    "Hello, World public!".to_string()
}

pub async fn get_offers(State(state): State<AppState>) -> Result<Json<Vec<Offer>>, AppError> {
    println!("Getting offers");
    let mut offers = state
        .database
        .query(
            "
            UPDATE transactions SET status = type::string('rejected') WHERE expiresAt < time::now() AND status = type::string('pending');
            FOR $id IN (SELECT VALUE id FROM offers WHERE status == 'stopped') {
                    IF COUNT(SELECT * FROM transactions WHERE status='pending' AND offerId = type::thing($id)) = 0 THEN {
                            UPDATE offers SET status = type::string('closed') WHERE id = type::thing($id);
                    } END;
            };
            SELECT id , (amount - MATH::SUM(SELECT VALUE amount+takerFee
            FROM transactions 
            WHERE offerId = $parent.id AND status != type::string('rejected'))) as amount, 
            cryptoType, currency, pricePerUnit, value, offerType, revTag, fee, status
            FROM offers 
            WHERE status = type::string('open') AND amount - MATH::SUM(SELECT VALUE amount+takerFee
            FROM transactions 
            WHERE offerId = $parent.id AND status != type::string('rejected')) > 0;
        ",
        )
        .await?;

    println!("Offers: {:?}", offers);
    let offers: Vec<Offer> = offers.take(2).map_err(AppError::from)?;
    println!("Offers: {:?}", offers);

    let offers_json = Json(offers);
    println!("Offers JSON: {:?}", offers_json);
    Ok(offers_json)
}

pub async fn get_deposit_address(
    State(state): State<AppState>,
) -> Result<Json<DepositAddressResponse>, AppError> {
    println!("Getting deposit address");
    let deposit_address = state.wallet_address.clone();
    Ok(Json(DepositAddressResponse {
        address: deposit_address,
    }))
}
