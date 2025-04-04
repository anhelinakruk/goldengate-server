use axum::{extract::State, routing::get, Json, Router};
use models::Offer;

use crate::AppState;

use super::AppError;

pub mod models;

pub fn router(app_state: &AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/offers", get(get_offers))
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
    let offers: Vec<Offer> = offers.take(0).map_err(AppError::from)?;
    println!("Offers: {:?}", offers);
    Ok(Json(offers))
}
