use std::fmt::{
    Display,
    Formatter,
};

use chrono::NaiveDateTime;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value;

use crate::deserialization::{
    iso_date_time,
    string_as_float,
    transfer_date,
};

/// # Account Data
/// A strongly typed representation of the account data returned by [/accounts](https://api.exchange.coinbase.com/accounts).
/// CBPro API reference: [Profiles](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getaccounts).
///
/// # JSON Input Example
///
/// ```ignore
///{
///     "id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "currency":"BTC",
///     "balance":"0.0000000000000000",
///     "hold":"0.0000000000000000",
///     "available":"0",
///     "profile_id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "trading_enabled":true
///}
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub id: String,
    pub currency: String,
    #[serde(with = "string_as_float")]
    pub balance: f64,
    #[serde(with = "string_as_float")]
    pub hold: f64,
    #[serde(with = "string_as_float")]
    pub available: f64,
    pub profile_id: String,
    pub trading_enabled: bool,
}

impl PartialEq<&str> for Account {
    fn eq(&self, other: &&str) -> bool {
        self.id.as_str() == *other
    }
}

///# Hold Data
/// A strongly typed representation of the hold data returned by [/accounts/{account_id}/holds](https://api.exchange.coinbase.com/accounts/{account_id}/holds).
/// CBPro API reference: [Holds](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getaccountholds).
///
/// # JSON Input Example
///
/// ```ignore
///{
///     "id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "created_at":"2022-01-20T00:00:00.000000Z",
///     "amount":"0.0000000000000000",
///     "ref":"deadbeef-dead-beef-dead-beefdeadbeex",
///     "type":"order"
///}
///```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hold {
    pub id: String,
    #[serde(with = "iso_date_time")]
    pub created_at: NaiveDateTime,
    pub amount: String,
    #[serde(rename = "ref")]
    pub ref_string: String,
    #[serde(rename = "type")]
    pub type_string: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ledger {
    pub id: String,
    #[serde(with = "string_as_float")]
    pub amount: f64,
    #[serde(with = "iso_date_time")]
    pub created_at: NaiveDateTime,
    #[serde(with = "string_as_float")]
    pub balance: f64,
    #[serde(flatten)]
    pub details: LedgerDetail,
}

/// todo!(LedgerDetail Variants) Needs an example for Fee, Rebate, and Conversion
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "details", rename_all = "lowercase")]
pub enum LedgerDetail {
    Match(MatchDetail),
    Transfer(TransferDetail),
    Fee(Value),
    Rebate(Value),
    Conversion(Value),
}

impl LedgerDetail {
    pub fn variant_name(&self) -> &str {
        match self {
            LedgerDetail::Match(_) => "match",
            LedgerDetail::Transfer(_) => "transfer",
            LedgerDetail::Fee(_) => "fee",
            LedgerDetail::Rebate(_) => "rebate",
            LedgerDetail::Conversion(_) => "conversion",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatchDetail {
    pub order_id: String,
    pub product_id: String,
    pub trade_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransferDetail {
    pub transfer_id: String,
    pub transfer_type: String,
}

impl Display for Ledger {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.id)?;
        writeln!(f, "{}", self.amount)?;
        writeln!(f, "{}", self.created_at)?;
        writeln!(f, "{}", self.balance)?;
        writeln!(f, "{:?}", self.details)
    }
}

///# Transfer Data
/// A strongly typed representation of the transfer data returned by [/accounts/{account_id}/transfers](https://api.exchange.coinbase.com/accounts/{account_id}/transfers).
///
/// CBPro API reference: [Transfers](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getaccounttransfers).
///
/// # JSON Input Example
///
/// ```ignore
/// {
///     "id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "type":"withdraw",
///     "created_at":"2021-09-13 00:00:00.000000+00",
///     "completed_at":"2021-09-13 00:00:00.000000+00",
///     "canceled_at":null,
///     "processed_at":"2021-09-13 00:00:00.00000+00",
///     "user_nonce":"1234567891011",
///     "amount":"0.00000000",
///     "details":
///     {
///		    "fee":"0.000000",
///		    "subtotal":"0.00",
///		    "sent_to_address":"0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF",
///		    "coinbase_account_id":"deadbeef-dead-beef-dead-beefdeadbeef",
///		    "coinbase_withdrawal_id":"bofadeeznutsdeadbeefligm",
///		    "coinbase_transaction_id":"bofadeeznutsdeadbeefligm",
///		    "crypto_transaction_hash":"bofadeeznutsdeadbeefligmaballzsugmamikehuntjennytaliabofadeeznut",
///		    "coinbase_payment_method_id":""
///     }
/// }
///```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transfer {
    pub id: String,
    #[serde(rename = "type")]
    pub type_string: String,
    #[serde(with = "transfer_date")]
    pub created_at: NaiveDateTime,
    #[serde(with = "transfer_date")]
    pub completed_at: NaiveDateTime,
    pub canceled_at: Option<String>,
    #[serde(with = "transfer_date")]
    pub processed_at: NaiveDateTime,
    pub user_nonce: Option<String>,
    #[serde(with = "string_as_float")]
    pub amount: f64,
    pub details: Details,
    pub idem: Option<String>,
}

///# Details Data
/// A strongly typed representation of the detail data within the [Transfer] struct.
///
/// CBPro API reference: [Transfers](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getaccounttransfers).
/// These details come in 1 of 2 forms, a withdrawal detail and a deposit detail struct.
///
/// todo!("Represent this enum as a single struct with optionals.")
///
/// # JSON Input Example
///
///```ignore
/// {
///     "fee":"0.000000",
///     "subtotal":"0.00",
///     "sent_to_address":"0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF",
///     "coinbase_account_id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "coinbase_withdrawal_id":"bofadeeznutsdeadbeefligm",
///     "coinbase_transaction_id":"bofadeeznutsdeadbeefligm",
///     "crypto_transaction_hash":"bofadeeznutsdeadbeefligmaballzsugmamikehuntjennytaliabofadeeznut",
///     "coinbase_payment_method_id":""
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Details {
    Withdraw(WithdrawDetails),
    Deposit(DepositDetails),
}

///# Withdrawal Details Data
/// A strongly typed representation of the withdrawal detail data within the [Transfer] struct.
///
/// CBPro API reference: [Transfers](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getaccounttransfers).
///
///
/// # JSON Input Example
///
///```ignore
/// {
///     "fee":"0.000000",
///     "subtotal":"0.00",
///     "sent_to_address":"0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF",
///     "coinbase_account_id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "coinbase_withdrawal_id":"bofadeeznutsdeadbeefligm",
///     "coinbase_transaction_id":"bofadeeznutsdeadbeefligm",
///     "crypto_transaction_hash":"bofadeeznutsdeadbeefligmaballzsugmamikehuntjennytaliabofadeeznut",
///     "coinbase_payment_method_id":""
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawDetails {
    #[serde(with = "string_as_float")]
    pub fee: f64,
    #[serde(with = "string_as_float")]
    pub subtotal: f64,
    pub sent_to_address: String,
    pub coinbase_account_id: String,
    pub coinbase_withdrawal_id: String,
    pub coinbase_transaction_id: String,
    pub crypto_transaction_hash: String,
    pub coinbase_payment_method_id: String,
}

///# Deposit Details Data
/// A strongly typed representation of the deposit detail data within the [Transfer] struct.
///
/// CBPro API reference: [Transfers](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getaccounttransfers).
///
///
/// # JSON Input Example
///
///```ignore
/// {
///     "fee":"0.000000",
///     "subtotal":"0.00",
///     "sent_to_address":"0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF",
///     "coinbase_account_id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "coinbase_withdrawal_id":"bofadeeznutsdeadbeefligm",
///     "coinbase_transaction_id":"bofadeeznutsdeadbeefligm",
///     "crypto_transaction_hash":"bofadeeznutsdeadbeefligmaballzsugmamikehuntjennytaliabofadeeznut",
///     "coinbase_payment_method_id":""
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositDetails {
    pub crypto_address: String,
    pub destination_tag: Option<String>,
    pub coinbase_account_id: String,
    pub destination_tag_name: String,
    pub crypto_transaction_id: String,
    pub coinbase_transaction_id: String,
    pub crypto_transaction_hash: String,
}

///# Wallet Data
/// A strongly typed representation of the data returned by [/coinbase-accounts](https://api.exchange.coinbase.com/coinbase-accounts).
///
/// CBPro API reference: [Wallets](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcoinbaseaccounts).
///
///
/// # JSON Input Example
///
///```ignore
/// {
///     "id":"deadbeef-dead-beef-dead-beefdeadbeef",
///     "name":"USD Wallet",
///     "balance":"0.00",
///     "currency":"USD",
///     "type":"fiat",
///     "primary":false,
///     "active":true,
///     "available_on_consumer":true,
///     "destination_tag_name":"STX Memo",
///     "destination_tag_regex":"^.{0,34}$",
///     "wire_deposit_information":
///     {
///         "account_number":null,
///         "routing_number":"021214891",
///         "bank_name":"Cross River Bank",
///         "bank_address":"885 Teaneck Road, Teaneck, NJ 07666",
///         "bank_country":
///         {
///             "code":"US",
///             "name":"United States"
///         },
///         "account_name":"Coinbase Inc",
///         "account_address":"100 Pine Street, Suite 1250, San Francisco, CA 94111",
///         "reference":"AIXNTKRQEXC"
///     },
///     "swift_deposit_information":null,
///     "hold_balance":"0.00",
///     "hold_currency":"USD"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Wallet {
    pub id: String,
    pub name: String,
    pub balance: String,
    pub currency: String,
    #[serde(rename = "type")]
    pub type_string: String,
    pub primary: bool,
    pub active: bool,
    pub available_on_consumer: bool,
    pub ready: Option<bool>,
    pub destination_tag_name: Option<String>,
    pub destination_tag_regex: Option<String>,
    pub wire_deposit_information: Option<WireDepositInformation>,
    pub swift_deposit_information: Option<SwiftDepositInformation>,
    pub sepa_deposit_information: Option<SepaDepositInformation>,
    pub uk_deposit_information: Option<UKDepositInformation>,
    #[serde(with = "string_as_float")]
    pub hold_balance: f64,
    pub hold_currency: String,
}

///# Wire Deposit Information Data
/// A strongly typed representation of the wire deposit detail data within the [Wallet] struct.
///
/// CBPro API reference: [Wallets](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcoinbaseaccounts).
///
///
/// # JSON Input Example
///
/// ```ignore
/// {
///     "account_number":null,
///     "routing_number":"021214891",
///     "bank_name":"Cross River Bank",
///     "bank_address":"885 Teaneck Road, Teaneck, NJ 07666",
///     "bank_country":
///     {
///         "code":"US",
///         "name":"United States"
///     },
///     "account_name":"Coinbase Inc",
///     "account_address":"100 Pine Street, Suite 1250, San Francisco, CA 94111",
///     "reference":"AIXNTKRQEXC"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct WireDepositInformation {
    pub account_number: Option<String>,
    pub routing_number: String,
    pub bank_name: String,
    pub bank_address: String,
    pub bank_country: BankCountry,
    pub account_name: String,
    pub account_address: String,
    pub reference: String,
}

///# Swift Deposit Information Data
/// A strongly typed representation of the SWIFT deposit detail data within the [Wallet] struct.
///
/// CBPro API reference: [Wallets](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcoinbaseaccounts).
///
///
/// # JSON Input Example
///
/// ```ignore
/// {
///     "account_number":null,
///     "bank_name":"Cross River Bank",
///     "bank_address":"885 Teaneck Road, Teaneck, NJ 07666",
///     "bank_country":
///     {
///         "code":"US",
///         "name":"United States"
///     },
///     "account_name":"Coinbase Inc",
///     "account_address":"100 Pine Street, Suite 1250, San Francisco, CA 94111",
///     "reference":"AIXNTKRQEXC"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SwiftDepositInformation {
    pub account_number: String,
    pub bank_name: String,
    pub bank_address: String,
    pub bank_country: BankCountry,
    pub account_name: String,
    pub account_address: String,
    pub reference: String,
}

///# Sepa Deposit Information Data
/// A strongly typed representation of the Sepa deposit detail data within the [Wallet] struct.
///
/// CBPro API reference: [Wallets](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcoinbaseaccounts).
///
///
/// # JSON Input Example
///
/// ```ignore
/// {
///     "iban": "iban-string",
///     "swift": "swift-string",
///     "bank_name":"Cross River Bank",
///     "bank_address":"885 Teaneck Road, Teaneck, NJ 07666",
///     "bank_country":
///     {
///         "code":"US",
///         "name":"United States"
///     },
///     "account_name":"Coinbase Inc",
///     "account_address":"100 Pine Street, Suite 1250, San Francisco, CA 94111",
///     "reference":"AIXNTKRQEXC"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct SepaDepositInformation {
    pub iban: String,
    pub swift: String,
    pub bank_name: String,
    pub bank_address: String,
    pub bank_country: BankCountry,
    pub account_name: String,
    pub account_address: String,
    pub reference: String,
}

///# UK Deposit Information Data
/// A strongly typed representation of the Sepa deposit detail data within the [Wallet] struct.
///
/// CBPro API reference: [Wallets](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcoinbaseaccounts).
///
///
/// # JSON Input Example
///
/// ```ignore
/// {
///     "sort_code": "sort_code-string",
///     "account_number": "account_number-string",
///     "bank_name":"Cross River Bank",
///     "account_name":"Coinbase Inc",
///     "reference":"AIXNTKRQEXC"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct UKDepositInformation {
    pub sort_code: String,
    pub account_number: String,
    pub bank_name: String,
    pub account_name: String,
    pub reference: String,
}

///# Bank Country Data
/// A strongly typed representation of the bank country data within the [Wallet] struct.
///
/// CBPro API reference: [Wallets](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getcoinbaseaccounts).
///
///
/// # JSON Input Example
///
/// ```ignore
/// {
///     "code":"US",
///     "name":"United States"
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct BankCountry {
    pub code: String,
    pub name: String,
}

/// # Fee Data
/// A strongly typed representation of the data returned by [/fees](https://api.exchange.coinbase.com/fees).
///
/// CBPro API reference: [Fees](https://api.exchange.coinbase.com/fees).
///
///
/// # JSON Input Example
///
/// ```ignore
/// {
///     taker_fee_rate: "0.006",
///     maker_fee_rate: "0.004",
///     usd_volume: "2854.82",
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Fees {
    #[serde(with = "string_as_float")]
    pub taker_fee_rate: f64,
    #[serde(with = "string_as_float")]
    pub maker_fee_rate: f64,
    #[serde(with = "string_as_float")]
    pub usd_volume: f64,
}
