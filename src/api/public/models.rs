use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use surrealdb::sql::Thing;

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Offer {
    #[serde_as(serialize_as = "DisplayFromStr")]
    pub id: Thing,
    #[serde(rename = "offerType")]
    pub offer_type: String,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: i128,
    pub currency: String,
    pub amount: i128,
    #[serde(rename = "cryptoType")]
    pub crypto_type: String,
    pub fee: i128,
    pub status: String,
    pub value: i128,
    #[serde(rename = "revTag")]
    pub rev_tag: String,
}
