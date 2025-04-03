use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOfferRequest {
    #[serde(rename = "offerType")]
    pub offer_type: String,
    pub amount: i128,
    pub fee: i128,
    #[serde(rename = "cryptoType")]
    pub crypto_type: String,
    pub currency: String,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: i128,
    pub value: i128,
    #[serde(rename = "revTag")]
    pub rev_tag: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTransactionRequest {
    pub offer_id: String,
    pub amount: i128,
    #[serde(rename = "cryptoType")]
    pub crypto_type: String,
    pub price: i128,
    pub currency: String,
    #[serde(rename = "takerFee")]
    pub taker_fee: i128,
    #[serde(rename = "makerFee")]
    pub maker_fee: i128,
    pub value: i128,
    #[serde(rename = "randomTitle")]
    pub random_title: String,
}
