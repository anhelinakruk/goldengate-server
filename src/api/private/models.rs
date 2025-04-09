use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use surrealdb::sql::Thing;

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

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTransactionRequest {
    #[serde(rename = "offerId")]
    #[serde_as(serialize_as = "DisplayFromStr")]
    pub offer_id: String,
    pub amount: i128,
    #[serde(rename = "cryptoType")]
    pub crypto_type: String,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: i128,
    pub currency: String,
    #[serde(rename = "takerFee")]
    pub taker_fee: i128,
    #[serde(rename = "makerFee")]
    pub maker_fee: i128,
    pub value: i128,
    #[serde(rename = "randomTitle")]
    pub random_title: String,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAggregatedFeeRequest {
    #[serde_as(serialize_as = "DisplayFromStr")]
    #[serde(rename = "offerId")]
    pub offer_id: String,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAggregatedFeeResponse {
    #[serde(rename = "aggregatedFee")]
    pub fee: i128,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmDepositRequest {
    #[serde(rename = "txHash")]
    pub tx_hash: String,
    pub amount: i128,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfirmDepositResponse {
    #[serde_as(serialize_as = "DisplayFromStr")]
    pub id: Thing,
    pub balance: i128,
}
