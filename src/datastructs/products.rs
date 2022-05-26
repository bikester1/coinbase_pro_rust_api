use chrono::NaiveDateTime;
use serde::{
    Deserialize,
    Serialize,
};

use crate::deserialization::{
    option_iso_date_time,
    option_string_as_float,
    string_as_float,
};

/// # Product Data
/// A strongly typed representation of product data returned by [/products](https://api.exchange.coinbase.com/products).
///
/// CBPro API reference: [Products](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getproducts).
///
///
/// # JSON Input Example
///
/// ```ignore
///{
///     id: "ETH-USD",
///     base_currency: "ETH",
///     quote_currency: "USD",
///     base_min_size: "0.00022",
///     base_max_size: "12000.0",
///     quote_increment: "0.01",
///     base_increment: "0.000000001",
///     display_name: "ETH/USD",
///     min_market_funds: "1.0",
///     max_market_funds: "20000000.0",
///     margin_enabled: false,
///     post_only: false,
///     limit_only: false,
///     cancel_only: false,
///     status: "online",
///     status_message: "",
///     trading_disabled: false,
///     fx_stablecoin: false,
///     max_slippage_percentage: "0.02",
///     auction_mode: false
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Product {
    pub id: String,
    pub base_currency: String,
    pub quote_currency: String,
    #[serde(with = "string_as_float")]
    pub base_min_size: f64,
    #[serde(with = "string_as_float")]
    pub base_max_size: f64,
    #[serde(with = "string_as_float")]
    pub quote_increment: f64,
    #[serde(with = "string_as_float")]
    pub base_increment: f64,
    pub display_name: String,
    #[serde(with = "string_as_float")]
    pub min_market_funds: f64,
    #[serde(with = "string_as_float")]
    pub max_market_funds: f64,
    pub margin_enabled: bool,
    pub post_only: bool,
    pub limit_only: bool,
    pub cancel_only: bool,
    pub status: String,
    pub status_message: String,
    pub trading_disabled: Option<bool>,
    pub fx_stablecoin: Option<bool>,
    #[serde(with = "option_string_as_float")]
    pub max_slippage_percentage: Option<f64>,
    pub auction_mode: bool,
}

/// # Product Book Data
/// A strongly typed representation of product book data returned by [/products/{product_id}/book](https://api.exchange.coinbase.com/products/{product_id}/book).
///
/// CBPro API reference: [Product Book](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getproductbook).
///
///
/// # JSON Input Example
///

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct ProductBook {
    pub bids: Vec<Orders>,
    pub asks: Vec<Orders>,
    pub sequence: f64,
    pub auction_mode: Option<bool>,
    pub auction: Option<Auction>,
}

/// # Currency Data
/// A strongly typed representation of currency data returned by [/currencies](https://api.exchange.coinbase.com/currencies) and [/currencies/{currency_id}](https://api.exchange.coinbase.com/currencies/{currency_id}).
///
/// CBPro API reference:
/// - [Currencies](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcurrencies).
/// - [Currency](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcurrency).
///
///
/// # JSON Input Example
///
/// ```ignore
///{
///     "id": "BTC",
///     "name": "Bitcoin",
///     "min_size": "0.00000001",
///     "status": "online",
///     "message": "",
///     "max_precision": "0.00000001",
///     "convertible_to": [],
///     "details":
///     {
///         "type_string": "crypto",
///         "symbol": "₿",
///         "network_confirmations": 3,
///         "sort_order": 20,
///         "crypto_address_link": "https://live.blockcypher.com/btc/address/{{address}}",
///         "crypto_transaction_link": "https://live.blockcypher.com/btc/tx/{{txId}}",
///         "push_payment_methods": ["crypto"],
///         "group_types": ["btc", "crypto"],
///         "display_name": None,
///         "processing_time_seconds": None,
///         "min_withdrawal_amount": 0.0001,
///         "max_withdrawal_amount": 2400.0
///     }
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub min_size: String,
    pub status: String,
    pub message: String,
    pub max_precision: String,
    pub convertible_to: Vec<String>,
    pub details: CurrencyDetails,
}

/// # Strongly Typed xxx
/// A strongly typed representation of xxx .
///
/// CBPro API reference: .
///
///
/// # JSON Input Example
///
/// ```ignore
///     id: string required
///     name: string required
///     min_size: string required
///     status: string required
///     message: string
///     max_precision: string required
///     convertible_to: array of strings
///     details: object required
///          type: string
///          symbol: string
///          network_confirmations: int32
///          sort_order: int32
///          crypto_address_link: string
///          crypto_transaction_link: string
///          push_payment_methods: array of strings
///          group_types: array of strings
///          display_name: string
///          processing_time_seconds: float
///          min_withdrawal_amount: double
///          max_withdrawal_amount: double
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct CurrencyDetails {
    #[serde(rename = "type")]
    pub type_string: Option<String>,
    pub symbol: Option<String>,
    pub network_confirmations: Option<i32>,
    pub sort_order: Option<i32>,
    pub crypto_address_link: Option<String>,
    pub crypto_transaction_link: Option<String>,
    pub push_payment_methods: Vec<String>,
    pub group_types: Vec<String>,
    pub display_name: Option<String>,
    pub processing_time_seconds: Option<f64>,
    pub min_withdrawal_amount: Option<f64>,
    pub max_withdrawal_amount: Option<f64>,
}

/// # Strongly Typed xxx
/// A strongly typed representation of xxx .
///
/// CBPro API reference: .
///
///
/// # JSON Input Example
///

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Orders {
    #[serde(with = "string_as_float")]
    pub price: f64,
    #[serde(with = "string_as_float")]
    pub size: f64,
    pub num_orders: i64,
}

/// # Strongly Typed xxx
/// A strongly typed representation of xxx .
///
/// CBPro API reference: .
///
///
/// # JSON Input Example
///

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct Auction {
    #[serde(with = "string_as_float")]
    pub open_price: f64,
    #[serde(with = "string_as_float")]
    pub open_size: f64,
    #[serde(with = "string_as_float")]
    pub best_bid_price: f64,
    #[serde(with = "string_as_float")]
    pub best_bid_size: f64,
    #[serde(with = "string_as_float")]
    pub best_ask_price: f64,
    #[serde(with = "string_as_float")]
    pub best_ask_size: f64,
    #[serde(with = "string_as_float")]
    pub auction_state: f64,
    pub can_open: Option<String>,
    #[serde(with = "option_iso_date_time")]
    pub time: Option<NaiveDateTime>,
}
