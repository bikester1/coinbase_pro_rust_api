use chrono::NaiveDateTime;
use serde::{
    Deserialize,
    Serialize,
};

use crate::deserialization::{
    iso_date_time,
    option_iso_date_time,
    option_string_as_float,
    string_as_float,
    transfer_date,
};

///trade_id: int32 required
///product_id: String required
///order_id: String required
///user_id: String required
///profile_id: String required
///liquidity: String required
///price: String required
///size: String required
///fee: String required
///created_at: String required
///side: String required
///settled: boolean required
///usd_volume: String required
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Fill {
    pub trade_id: u64,
    pub product_id: String,
    pub order_id: String,
    pub user_id: String,
    pub profile_id: String,
    pub liquidity: String,
    #[serde(with = "string_as_float")]
    pub price: f64,
    #[serde(with = "string_as_float")]
    pub size: f64,
    #[serde(with = "string_as_float")]
    pub fee: f64,
    pub created_at: String,
    pub side: String,
    pub settled: bool,
    #[serde(with = "string_as_float")]
    pub usd_volume: f64,
}

/// {
///     "created_at": "2022-03-01T17:50:06.65121Z",
///     "executed_value": "0.0000000000000000",
///     "expire_time": "2022-03-02 17:50:06.66",
///     "fill_fees": "0.0000000000000000",
///     "filled_size": "0.00000000",
///     "id": "c714c4f6-1296-4451-89ca-040b3cfa8631",
///     "post_only": false,
///     "price": "3115.19000000",
///     "product_id": "ETH-USD",
///     "profile_id": "c37debbf-a41c-496e-a8b9-a85e6d3ef4ff",
///     "settled": false,
///     "side": "sell",
///     "size": "0.00500772",
///     "status": "open",
///     "time_in_force": "GTT",
///     "type": "limit",
/// }
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    #[serde(with = "iso_date_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "string_as_float")]
    pub executed_value: f64,
    #[serde(with = "transfer_date")]
    pub expire_time: NaiveDateTime,
    #[serde(with = "string_as_float")]
    pub fill_fees: f64,
    #[serde(with = "string_as_float")]
    pub filled_size: f64,
    pub id: String,
    pub post_only: bool,
    #[serde(with = "string_as_float")]
    pub price: f64,
    pub product_id: String,
    pub profile_id: String,
    pub settled: bool,
    pub side: String,
    #[serde(with = "string_as_float")]
    pub size: f64,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub type_string: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "time_in_force", content = "cancel_after")]
pub enum TimeInForce {
    // Good til canceled
    GTC,
    // Good til time
    GTT(CancelAfter),
    // Immediate or cancel
    IOC,
    // Fill or kill
    FOK,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TimeInForceResponse {
    // Good til canceled
    GTC,
    // Good til time
    GTT,
    // Immediate or cancel
    IOC,
    // Fill or kill
    FOK,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    BUY,
    SELL,
}

impl Default for Side {
    fn default() -> Self {
        Side::BUY
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SelfTradePrevention {
    DecreaseAndCancel,
    CancelOldest,
    CancelNewest,
    CancelBoth,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MarketOrderValue {
    Size(f64),
    Funds(f64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase", tag = "stop", content = "stop_price")]
pub enum Stop {
    #[serde(with = "string_as_float")]
    Loss(f64),
    #[serde(with = "string_as_float")]
    Entry(f64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CancelAfter {
    Min,
    Hour,
    Day,
}

pub trait CoinbaseOrder {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarketOrder {
    product_id: String,
    side: Side,
    market_order_details: MarketOrderValue,
    profile_id: Option<String>,
    self_trade_prevention: Option<SelfTradePrevention>,
    client_oid: Option<String>,
}

impl CoinbaseOrder for MarketOrder {}

impl MarketOrder {
    pub fn new(product_id: String, side: Side, value: MarketOrderValue) -> Self {
        Self {
            product_id,
            side,
            market_order_details: value,
            profile_id: None,
            self_trade_prevention: None,
            client_oid: None,
        }
    }
}

pub fn not(value: &bool) -> bool {
    !value.clone()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LimitOrder {
    product_id: String,
    side: Side,
    #[serde(with = "string_as_float")]
    price: f64,
    #[serde(with = "string_as_float")]
    size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    self_trade_prevention: Option<SelfTradePrevention>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_oid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    stop: Option<Stop>,
    #[serde(skip_serializing_if = "not")]
    post_only: bool,
}

impl CoinbaseOrder for LimitOrder {}

impl LimitOrder {
    pub fn new(product_id: String, side: Side, price: f64, size: f64) -> Self {
        Self {
            product_id,
            side,
            price,
            size,
            profile_id: None,
            self_trade_prevention: None,
            client_oid: None,
            time_in_force: None,
            stop: None,
            post_only: false,
        }
    }

    pub fn set_profile_id(mut self, profile_id: Option<String>) -> Self {
        self.profile_id = profile_id;
        self
    }

    pub fn profile_id(&self) -> &Option<String> {
        &self.profile_id
    }

    pub fn set_self_trade_prevention(
        mut self,
        self_trade_prevention: Option<SelfTradePrevention>,
    ) -> Self {
        self.self_trade_prevention = self_trade_prevention;
        self
    }

    pub fn self_trade_prevention(&self) -> &Option<SelfTradePrevention> {
        &self.self_trade_prevention
    }

    pub fn set_post_only(mut self, post_only: bool) -> Self {
        self.post_only = post_only;
        self
    }

    pub fn post_only(&self) -> &bool {
        &self.post_only
    }

    pub fn set_time_in_force(mut self, time_in_force: Option<TimeInForce>) -> Self {
        self.time_in_force = time_in_force;
        self
    }

    pub fn time_in_force(&self) -> &Option<TimeInForce> {
        &self.time_in_force
    }

    pub fn set_stop(mut self, stop: Option<Stop>) -> Self {
        self.stop = stop;
        self
    }

    pub fn stop(&self) -> &Option<Stop> {
        &self.stop
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewOrderResponse {
    pub id: String,
    #[serde(default, with = "option_string_as_float")]
    pub price: Option<f64>,
    #[serde(default)]
    pub size: Option<String>,
    pub product_id: String,
    #[serde(default)]
    pub profile_id: Option<String>,
    pub side: Side,
    #[serde(default, with = "option_string_as_float")]
    pub funds: Option<f64>,
    #[serde(default, with = "option_string_as_float")]
    pub specified_funds: Option<f64>,
    #[serde(rename = "type")]
    pub type_string: String,
    #[serde(default)]
    pub time_in_force: Option<TimeInForceResponse>,
    #[serde(with = "option_iso_date_time", default)]
    pub expire_time: Option<NaiveDateTime>,
    #[serde(with = "iso_date_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "option_iso_date_time", default)]
    pub done_at: Option<NaiveDateTime>,
    #[serde(default)]
    pub done_reason: Option<String>,
    #[serde(default)]
    pub reject_reason: Option<String>,
    #[serde(with = "string_as_float")]
    pub fill_fees: f64,
    #[serde(with = "string_as_float")]
    pub filled_size: f64,
    #[serde(default, with = "option_string_as_float")]
    pub executed_value: Option<f64>,
    pub status: String,
    pub settled: bool,
    #[serde(default)]
    pub stop: Option<String>,
    #[serde(default, with = "option_string_as_float")]
    pub stop_price: Option<f64>,
    #[serde(default, with = "option_string_as_float")]
    pub funding_amount: Option<f64>,
    #[serde(default)]
    pub client_oid: Option<String>,
}
