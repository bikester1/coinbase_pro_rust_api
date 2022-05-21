use serde::{
    Deserialize,
    Serialize,
};

use crate::deserialization::{
    iso_date_time,
    string_as_float,
};

///
/// Request
/// ```[ignore]
/// {
///     "type": "subscribe",
///     "channels": [{ "name": "heartbeat", "product_ids": ["ETH-EUR"] }]
/// }
/// ```
///
/// Heartbeat message
/// ```[ignore]
/// {
///     "type": "heartbeat",
///     "sequence": 90,
///     "last_trade_id": 20,
///     "product_id": "BTC-USD",
///     "time": "2014-11-07T08:19:28.464459Z"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[non_exhaustive]
pub enum WebsocketMessage {
    Subscribe(SubscribeRequest),
    Heartbeat(HeartbeatMessage),
    Subscriptions(SubscriptionsMessage),
    Status(StatusMessage),
    Ticker(TickerMessage),
    Snapshot(Level2Snapshot),
    L2Update(Level2Update),
    Received(ReceivedMessage),
    Open(OpenMessage),
    Match(MatchMessage),
    Done(DoneMessage),
    Change(ChangeMessage),
    Activate(ActivateMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "name")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[non_exhaustive]
pub enum Channel {
    Heartbeat(HeartbeatChannel),
    Status(StatusChannel),
    Ticker(TickerChannel),
    Level2(Level2Channel),
    Full(FullChannel),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribeRequest {
    pub channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscriptionsMessage {
    pub channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeartbeatChannel {
    pub product_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Level2Channel {
    pub product_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerChannel {
    pub product_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FullChannel {
    pub product_ids: Vec<String>,
}

/// // Request
/// ```[ignore]
/// {
///     "type": "subscribe",
///     "channels": [{ "name": "status"}]
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusChannel {}

///
/// ```[ignore]
/// {
///     "type": "status",
///     "products": [
///         {
///             "id": "BTC-USD",
///             "base_currency": "BTC",
///             "quote_currency": "USD",
///             "base_min_size": "0.001",
///             "base_max_size": "70",
///             "base_increment": "0.00000001",
///             "quote_increment": "0.01",
///             "display_name": "BTC/USD",
///             "status": "online",
///             "status_message": null,
///             "min_market_funds": "10",
///             "max_market_funds": "1000000",
///             "post_only": false,
///             "limit_only": false,
///             "cancel_only": false,
///             "fx_stablecoin": false
///         }
///     ],
///     "currencies": [
///         {
///             "id": "USD",
///             "name": "United States Dollar",
///             "min_size": "0.01000000",
///             "status": "online",
///             "status_message": null,
///             "max_precision": "0.01",
///             "convertible_to": ["USDC"],
///             "details": {}
///         },
///         {
///             "id": "USDC",
///             "name": "USD Coin",
///             "min_size": "0.00000100",
///             "status": "online",
///             "status_message": null,
///             "max_precision": "0.000001",
///             "convertible_to": ["USD"],
///             "details": {}
///         },
///         {
///             "id": "BTC",
///             "name": "Bitcoin",
///             "min_size":" 0.00000001",
///             "status": "online",
///             "status_message": null,
///             "max_precision": "0.00000001",
///             "convertible_to": [],
///             "details": {}
///         }
///     ]
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusMessage {
    pub products: Vec<StatusProduct>,
    pub currencies: Vec<StatusCurrency>,
}

///
/// ```[ignore]
/// {
///     "id": "BTC-USD",
///     "base_currency": "BTC",
///     "quote_currency": "USD",
///     "base_min_size": "0.001",
///     "base_max_size": "70",
///     "base_increment": "0.00000001",
///     "quote_increment": "0.01",
///     "display_name": "BTC/USD",
///     "status": "online",
///     "status_message": null,
///     "min_market_funds": "10",
///     "max_market_funds": "1000000",
///     "post_only": false,
///     "limit_only": false,
///     "cancel_only": false,
///     "fx_stablecoin": false
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusProduct {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
    #[serde(with = "string_as_float")]
    pub base_min_size: f64,
    #[serde(with = "string_as_float")]
    pub base_max_size: f64,
    #[serde(with = "string_as_float")]
    pub base_increment: f64,
    #[serde(with = "string_as_float")]
    pub quote_increment: f64,
    pub display_name: String,
    pub status: String,
    pub status_message: Option<String>,
    #[serde(with = "string_as_float")]
    pub min_market_funds: f64,
    #[serde(with = "string_as_float")]
    pub max_market_funds: f64,
    pub post_only: bool,
    pub limit_only: bool,
    pub cancel_only: bool,
    pub fx_stablecoin: bool,
}

///
/// ```[ignore]
/// {
///     "id": "USDC",
///     "name": "USD Coin",
///     "min_size": "0.00000100",
///     "status": "online",
///     "status_message": null,
///     "max_precision": "0.000001",
///     "convertible_to": ["USD"],
///     "details": {}
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusCurrency {
    pub id: String,
    pub name: String,
    #[serde(with = "string_as_float")]
    pub min_size: f64,
    pub status: String,
    pub status_messag: Option<String>,
    #[serde(with = "string_as_float")]
    pub max_precision: f64,
    pub convertible_to: Vec<String>,
    pub details: StatusDetails,
}

///
/// ```[ignore]
/// {
///     "type":"crypto",
///     "symbol":"",
///     "network_confirmations":35,
///     "sort_order":421,
///     "crypto_address_link":"https://etherscan.io/token/0x799ebfABE77a6E34311eeEe9825190B9ECe32824?a={{address}}",
///     "crypto_transaction_link":"https://etherscan.io/tx/0x{{txId}}",
///     "push_payment_methods":["crypto"],
///     "min_withdrawal_amount":1e-18,
///     "max_withdrawal_amount":125000
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum StatusDetails {
    Crypto(CryptoDetails),
    Fiat(FiatDetails),
}

///
/// ```[ignore]
/// {
///     "type":"crypto",
///     "symbol":"",
///     "network_confirmations":35,
///     "sort_order":421,
///     "crypto_address_link":"https://etherscan.io/token/0x799ebfABE77a6E34311eeEe9825190B9ECe32824?a={{address}}",
///     "crypto_transaction_link":"https://etherscan.io/tx/0x{{txId}}",
///     "push_payment_methods":["crypto"],
///     "min_withdrawal_amount":1e-18,
///     "max_withdrawal_amount":125000
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CryptoDetails {
    pub symbol: String,
    pub network_confirmations: u64,
    pub sort_order: u64,
    pub crypto_address_link: String,
    pub crypto_transaction_link: String,
    pub push_payment_methods: Vec<String>,
    pub min_withdrawal_amount: f64,
    pub max_withdrawal_amount: f64,
}

///
/// ```[ignore]
/// {
///     "type":"fiat",
///     "symbol":"$",
///     "network_confirmations":0,
///     "sort_order":1,
///     "crypto_address_link":"",
///     "crypto_transaction_link":"",
///     "push_payment_methods":["bank_wire","fedwire","swift_bank_account","intra_bank_account"],
///     "group_types":["fiat","usd"],
///     "display_name":"US Dollar"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FiatDetails {
    pub symbol: String,
    pub network_confirmations: u64,
    pub sort_order: u64,
    pub crypto_address_link: String,
    pub crypto_transaction_link: String,
    pub push_payment_methods: Vec<String>,
    pub group_types: Vec<String>,
    pub display_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HeartbeatMessage {
    pub sequence: u64,
    pub last_trade_id: u64,
    pub product_id: String,
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
}

///
/// ```[ignore]
/// {
///     "type": "ticker",
///     "trade_id": 20153558,
///     "sequence": 3262786978,
///     "time": "2017-09-02T17:05:49.250000Z",
///     "product_id": "BTC-USD",
///     "price": "4388.01000000",
///     "side": "buy", // taker side
///     "last_size": "0.03000000",
///     "best_bid": "4388",
///     "best_ask": "4388.01"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerMessage {
    pub trade_id: u64,
    pub sequence: u64,
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
    pub product_id: String,
    #[serde(with = "string_as_float")]
    pub price: f64,
    pub side: String,
    #[serde(with = "string_as_float")]
    pub last_size: f64,
    #[serde(with = "string_as_float")]
    pub best_bid: f64,
    #[serde(with = "string_as_float")]
    pub best_ask: f64,
}

///
/// ```[ignore]
/// {
///     "type": "snapshot",
///     "product_id": "BTC-USD",
///     "bids": [["10101.10", "0.45054140"]],
///     "asks": [["10102.55", "0.57753524"]]
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Level2Snapshot {
    pub product_id: String,
    pub bids: Vec<Quote>,
    pub asks: Vec<Quote>,
}

///
/// ```[ignore]
/// {
///   "type": "l2update",
///   "product_id": "BTC-USD",
///   "time": "2019-08-14T20:42:27.265Z",
///   "changes": [
///     [
///       "buy",
///       "10101.80000000",
///       "0.162567"
///     ]
///   ]
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Level2Update {
    pub product_id: String,
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
    pub changes: Vec<Level2Change>,
}

///
/// ```[ignore]
/// [
///   "buy",
///   "10101.80000000",
///   "0.162567"
/// ]
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Level2Change {
    pub side: String,
    #[serde(with = "string_as_float")]
    pub price: f64,
    #[serde(with = "string_as_float")]
    pub size: f64,
}

///
/// ```[ignore]
/// {
///     "type": "received",
///     "time": "2014-11-07T08:19:27.028459Z",
///     "product_id": "BTC-USD",
///     "sequence": 10,
///     "order_id": "d50ec984-77a8-460a-b958-66f114b0de9b",
///     "size": "1.34",
///     "price": "502.1",
///     "side": "buy",
///     "order_type": "limit"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReceivedMessage {
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
    pub product_id: String,
    pub sequence: u64,
    pub order_id: String,
    #[serde(flatten)]
    pub order: Order,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "order_type")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum Order {
    Limit(LimitOrder),
    Market(MarketOrder),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LimitOrder {
    #[serde(with = "string_as_float")]
    pub size: f64,
    #[serde(with = "string_as_float")]
    pub price: f64,
    pub side: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarketOrder {
    #[serde(default)]
    #[serde(with = "string_as_float")]
    pub funds: f64,
    pub side: String,
}

///
/// ```[ignore]
/// {
///     "type": "open",
///     "time": "2014-11-07T08:19:27.028459Z",
///     "product_id": "BTC-USD",
///     "sequence": 10,
///     "order_id": "d50ec984-77a8-460a-b958-66f114b0de9b",
///     "price": "200.2",
///     "remaining_size": "1.00",
///     "side": "sell"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenMessage {
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
    pub product_id: String,
    pub sequence: u64,
    pub order_id: String,
    #[serde(with = "string_as_float")]
    pub price: f64,
    #[serde(with = "string_as_float")]
    pub remaining_size: f64,
    pub side: String,
}

///
/// ```[ignore]
/// {
///     "type": "done",
///     "time": "2014-11-07T08:19:27.028459Z",
///     "product_id": "BTC-USD",
///     "sequence": 10,
///     "price": "200.2",
///     "order_id": "d50ec984-77a8-460a-b958-66f114b0de9b",
///     "reason": "filled", // or "canceled"
///     "side": "sell",
///     "remaining_size": "0"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DoneMessage {
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
    pub product_id: String,
    pub sequence: u64,
    #[serde(default)]
    #[serde(with = "string_as_float")]
    pub price: f64,
    pub order_id: String,
    pub reason: String, // or "canceled"
    pub side: String,
    #[serde(default)]
    #[serde(with = "string_as_float")]
    pub remaining_size: f64,
}

///
/// ```[ignore]
/// {
///     "type": "match",
///     "trade_id": 10,
///     "sequence": 50,
///     "maker_order_id": "ac928c66-ca53-498f-9c13-a110027a60e8",
///     "taker_order_id": "132fb6ae-456b-4654-b4e0-d681ac05cea1",
///     "time": "2014-11-07T08:19:27.028459Z",
///     "product_id": "BTC-USD",
///     "size": "5.23512",
///     "price": "400.23",
///     "side": "sell"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatchMessage {
    pub trade_id: u64,
    pub sequence: u64,
    pub maker_order_id: String,
    pub taker_order_id: String,
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
    pub product_id: String,
    #[serde(with = "string_as_float")]
    pub size: f64,
    #[serde(with = "string_as_float")]
    pub price: f64,
    pub side: String,
}

///
///
/// ```[ignore]
/// {
///     "type": "change",
///     "time": "2014-11-07T08:19:27.028459Z",
///     "sequence": 80,
///     "order_id": "ac928c66-ca53-498f-9c13-a110027a60e8",
///     "product_id": "BTC-USD",
///     "new_size": "5.23512",
///     "old_size": "12.234412",
///     "price": "400.23",
///     "side": "sell"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChangeMessage {
    #[serde(with = "iso_date_time")]
    pub time: chrono::NaiveDateTime,
    pub sequence: u64,
    pub order_id: String,
    pub product_id: String,
    #[serde(with = "string_as_float")]
    pub new_size: f64,
    #[serde(with = "string_as_float")]
    pub old_size: f64,
    #[serde(with = "string_as_float")]
    pub price: f64,
    pub side: String,
}

/// # Websocket Activate Message
/// An activate message is sent when a stop order is placed.
/// When the stop is triggered the order will be placed and go through the order lifecycle.
///
/// This struct should never need to be created on it's own but can be returned from the full channel websocket feed.
///
/// ```[ignore]
/// {
///   "type": "activate",
///   "product_id": "test-product",
///   "timestamp": "1483736448.299000",
///   "user_id": "12",
///   "profile_id": "30000727-d308-cf50-7b1c-c06deb1934fc",
///   "order_id": "7b52009b-64fd-0a2a-49e6-d8a939753077",
///   "stop_type": "entry",
///   "side": "buy",
///   "stop_price": "80",
///   "size": "2",
///   "funds": "50",
///   "private": true
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivateMessage {
    pub product_id: String,
    pub timestamp: String,
    pub user_id: String,
    pub profile_id: String,
    pub order_id: String,
    pub stop_type: String,
    pub side: String,
    #[serde(with = "string_as_float")]
    pub stop_price: f64,
    #[serde(with = "string_as_float")]
    pub size: f64,
    #[serde(with = "string_as_float")]
    pub funds: f64,
    pub private: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Quote {
    #[serde(with = "string_as_float")]
    price: f64,
    #[serde(with = "string_as_float")]
    size: f64,
}
