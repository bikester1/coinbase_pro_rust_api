//! # Coinbase Pro API
//! ![Build](https://img.shields.io/github/workflow/status/bikester1/coinbase_pro_rust_api/Rust/main?style=for-the-badge)
//! ![coverage](https://img.shields.io/badge/Coverage-82%25-yellow?style=for-the-badge)
//!
//!
//! coinbase_pro is an api for getting market data from the coinbase pro public API.
//! This crate aims to be a simple lightweight interface for making requests to coinbase's API.
//! This crate also aims to make available abstractions at the lowest level possible.
//! This allows users to specify how the responses get parsed.
//!
//! ## Quickstart Info
//!
//! This api has a main client struct called [api::CBProAPI]. This struct is like a reqwest struct and
//! can be cheaply copied, cloned, and passed between threads. Internally it implements its
//! state utilizing [std::sync::Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html)
//! and [tokio::sync::Mutex](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html).
//!
//!
//! ## Future Proofing
//!
//! In addition to the standard usage of this api through [api::CBProAPI], this crate exposes a low level
//! [requests::CBRequestBuilder] that allows additional endpoints and custom deserialization if coinbase
//! ever changed their api, endpoints, or data formats.
//!
//!
//! ## Examples
//!
//! ### Basic Usage
//! ```
//! use coinbase_pro::api::CBProAPI;
//!
//! #[tokio::test]
//! async fn get_product() {
//!     let api = CBProAPI::default();
//!     let product = api.get_product("ETH-USD".to_string()).await.unwrap();
//!
//!     assert_eq!(product.display_name, "ETH-USD");
//! }
//! ```
//!
//! ### Websocket
//! ```
//! use coinbase_pro::api::CBProAPI;
//! use coinbase_pro::api::SubscriptionBuilder;
//!
//! #[tokio::test]
//! async fn subscribe() {
//!     let mut api = CBProAPI::default();
//!     let subscribe_message = SubscriptionBuilder::new()
//!         .subscribe_to_heartbeat("ETH-USD".to_string())
//!         .build();
//!
//!     api.subscribe_to_websocket(subscribe_message).await.unwrap();
//!     
//!     let response = api.read_websocket().await.unwrap();
//! }
//! ```
extern crate core;

pub mod api;
mod deserialization;
mod errors;
pub mod requests;
mod websocket_lite;

pub mod datastructs;
mod mocked;
pub mod order_book;

#[cfg(all(test, feature = "mock"))]
mod tests {

    use std::ops::Deref;

    use log::LevelFilter;

    use reqwest::header::HeaderValue;

    use simple_logger::SimpleLogger;
    use tokio::io::{
        AsyncReadExt,
        AsyncWriteExt,
    };

    use crate::api::{
        APIKeyData,
        CBProAPI,
        Level,
        SubscriptionBuilder,
    };

    use crate::datastructs::orders::{
        MarketOrder,
        MarketOrderValue,
        Side,
    };
    use crate::errors::Error;
    use crate::mocked::{
        CallInfo,
        MockClient,
        MockHeaderMap,
        MockRequestBuilder,
        MockResponse,
        MockTcpStream,
        MockTlsStream,
    };

    #[tokio::test]
    async fn mocked_api_coinbase_server_error() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
            {
                "message": "error message"
            }"#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api.get_all_products().await;

        assert!(output.is_err());

        match output.unwrap_err() {
            Error::CBProServerErrorVariant(message) => {
                assert_eq!(message.message, "error message");
            }
            _ => {
                unreachable!();
            }
        }
    }

    #[tokio::test]
    async fn mocked_api_get_all_products_minimum_required_response() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
        [
            {
                "id":"UMA-EUR",
                "base_currency":"UMA",
                "quote_currency":"EUR",
                "base_min_size":"0.062",
                "base_max_size":"27000",
                "quote_increment":"0.001",
                "base_increment":"0.001",
                "display_name":"UMA/EUR",
                "min_market_funds":"0.84",
                "max_market_funds":"190000",
                "margin_enabled":false,
                "post_only":false,
                "limit_only":false,
                "cancel_only":false,
                "status":"online",
                "status_message":"",
                "auction_mode":false
            }
        ]"#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api.get_all_products().await.unwrap();

        let prod = output.get(0).unwrap();

        assert_eq!(1, output.len());
        assert_eq!(prod.id, "UMA-EUR");
        assert_eq!(prod.base_currency, "UMA");
        assert_eq!(prod.quote_currency, "EUR");
        assert_eq!(prod.base_min_size, 0.062);
        assert_eq!(prod.base_max_size, 27000.0);
        assert_eq!(prod.quote_increment, 0.001);
        assert_eq!(prod.base_increment, 0.001);
        assert_eq!(prod.display_name, "UMA/EUR");
        assert_eq!(prod.min_market_funds, 0.84);
        assert_eq!(prod.max_market_funds, 190000.0);
        assert_eq!(prod.margin_enabled, false);
        assert_eq!(prod.post_only, false);
        assert_eq!(prod.limit_only, false);
        assert_eq!(prod.cancel_only, false);
        assert_eq!(prod.status, "online");
        assert_eq!(prod.status_message, "");
        assert_eq!(prod.auction_mode, false);
    }

    #[tokio::test]
    async fn mocked_api_get_all_products_json_header() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1
            .expect_text()
            .return_once(|| Ok(r#"[{"id":"Causes Error",}]"#.to_string()));

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        match api.get_all_products().await {
            Ok(_) => {}
            Err(_) => {}
        };

        let header_calls: Vec<CallInfo> = mock_request_builder
            .call_info
            .deref()
            .take()
            .into_iter()
            .filter(|x| {
                x.method_name == "header"
                    && x.called_with(("key", "\"Content-Type\""))
                    && x.called_with(("value", "\"application/json\""))
            })
            .collect();

        assert_eq!(1, header_calls.len());
    }

    #[tokio::test]
    async fn mocked_api_get_all_products_all_fields() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
        [
            {
                "id":"UMA-EUR",
                "base_currency":"UMA",
                "quote_currency":"EUR",
                "base_min_size":"0.062",
                "base_max_size":"27000",
                "quote_increment":"0.001",
                "base_increment":"0.001",
                "display_name":"UMA/EUR",
                "min_market_funds":"0.84",
                "max_market_funds":"190000",
                "margin_enabled":false,
                "post_only":false,
                "limit_only":false,
                "cancel_only":false,
                "status":"online",
                "status_message":"",
                "auction_mode":false,
                "trading_disabled": null,
                "fx_stablecoin": false,
                "max_slippage_percentage": "0.00000001"
            }
        ]"#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api.get_all_products().await.unwrap();

        let prod = output.get(0).unwrap();

        assert_eq!(prod.trading_disabled, None);
        assert_eq!(prod.fx_stablecoin, Some(false));
        assert_eq!(prod.max_slippage_percentage, Some(0.00000001));
    }

    #[tokio::test]
    async fn mocked_api_get_product_minimum_required_response() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
            {
                "id":"UMA-EUR",
                "base_currency":"UMA",
                "quote_currency":"EUR",
                "base_min_size":"0.062",
                "base_max_size":"27000",
                "quote_increment":"0.001",
                "base_increment":"0.001",
                "display_name":"UMA/EUR",
                "min_market_funds":"0.84",
                "max_market_funds":"190000",
                "margin_enabled":false,
                "post_only":false,
                "limit_only":false,
                "cancel_only":false,
                "status":"online",
                "status_message":"",
                "auction_mode":false
            }"#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api.get_product("MyProduct".to_string()).await.unwrap();

        let prod = output;

        assert_eq!(prod.id, "UMA-EUR");
        assert_eq!(prod.base_currency, "UMA");
        assert_eq!(prod.quote_currency, "EUR");
        assert_eq!(prod.base_min_size, 0.062);
        assert_eq!(prod.base_max_size, 27000.0);
        assert_eq!(prod.quote_increment, 0.001);
        assert_eq!(prod.base_increment, 0.001);
        assert_eq!(prod.display_name, "UMA/EUR");
        assert_eq!(prod.min_market_funds, 0.84);
        assert_eq!(prod.max_market_funds, 190000.0);
        assert_eq!(prod.margin_enabled, false);
        assert_eq!(prod.post_only, false);
        assert_eq!(prod.limit_only, false);
        assert_eq!(prod.cancel_only, false);
        assert_eq!(prod.status, "online");
        assert_eq!(prod.status_message, "");
        assert_eq!(prod.auction_mode, false);
    }

    #[tokio::test]
    async fn mocked_api_get_product_path_check() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1
            .expect_text()
            .return_once(|| Ok(r#"{"id":"UMA-EUR"}"#.to_string()));

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client.clone());

        api.get_product("MyProduct".to_string()).await.unwrap();

        assert!(mock_client
            .requested_url
            .borrow_mut()
            .contains("/MyProduct"));
    }

    #[tokio::test]
    async fn mocked_api_get_product_book_minimum_response() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
            {
                "bids": [],
                "asks": [],
                "sequence": 100
            }
        "#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api
            .get_product_book("MyProduct".to_string(), Some(Level::One))
            .await
            .unwrap();

        assert_eq!(output.bids.len(), 0);
        assert_eq!(output.asks.len(), 0);
        assert_eq!(output.sequence, 100.0);
        assert_eq!(output.auction_mode, None);
        assert!(output.auction.is_none());
    }

    #[tokio::test]
    async fn mocked_api_get_product_book_minimum_response_2() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
            {
                "bids": [{"price":"1","size":"1","num_orders":1}],
                "asks": [{"price":"1","size":"1","num_orders":1}],
                "sequence": 100,
                "auction_mode": true,
                "auction": {
                    "open_price": "0.1",
                    "open_size": "0.1",
                    "best_bid_price": "0.1",
                    "best_bid_size": "0.1",
                    "best_ask_price": "0.1",
                    "best_ask_size": "0.1",
                    "auction_state": "0.1"
                }
            }
        "#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api
            .get_product_book("MyProduct".to_string(), Some(Level::One))
            .await
            .unwrap();

        assert_eq!(output.bids.len(), 1);
        assert_eq!(output.asks.len(), 1);
        assert_eq!(output.sequence, 100.0);
        assert_eq!(output.auction_mode, Some(true));
        assert!(output.auction.is_some());
        assert_eq!(output.auction.as_ref().unwrap().open_price, 0.1);
        assert_eq!(output.auction.as_ref().unwrap().open_size, 0.1);
        assert_eq!(output.auction.as_ref().unwrap().best_bid_price, 0.1);
        assert_eq!(output.auction.as_ref().unwrap().best_bid_size, 0.1);
        assert_eq!(output.auction.as_ref().unwrap().best_ask_price, 0.1);
        assert_eq!(output.auction.as_ref().unwrap().best_ask_size, 0.1);
        assert_eq!(output.auction.as_ref().unwrap().auction_state, 0.1);
    }

    #[tokio::test]
    async fn mocked_api_get_fees() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
            {
                "taker_fee_rate": "0.1",
                "maker_fee_rate": "0.1",
                "usd_volume": "0.1"
            }
        "#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        let output = api.get_fees(account).await.unwrap();

        assert_eq!(output.maker_fee_rate, 0.1);
        assert_eq!(output.taker_fee_rate, 0.1);
        assert_eq!(output.usd_volume, 0.1);
    }

    #[tokio::test]
    async fn mocked_api_get_accounts() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"[
            {
                "id": "id",
                "currency": "currency",
                "balance": "0.1",
                "available": "0.1",
                "hold": "0.1",
                "profile_id": "profile_id",
                "trading_enabled": false
            }
        ]"#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        let output = api.get_accounts(account).await.unwrap();

        let acct = output.get(0).unwrap();

        assert_eq!(output.len(), 1);
        assert_eq!(acct.id, "id");
        assert_eq!(acct.hold, 0.1);
    }

    #[tokio::test]
    async fn mocked_api_get_account() {
        let mut respone1 = MockResponse::new();
        let respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
            {
                "id": "id",
                "currency": "currency",
                "balance": "0.1",
                "available": "0.1",
                "hold": "0.1",
                "profile_id": "profile_id",
                "trading_enabled": false
            }
        "#
            .to_string())
        });

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        let output = api.get_account(account, "id").await.unwrap();

        let acct = output;
        assert_eq!(acct.id, "id");
        assert_eq!(acct.hold, 0.1);
    }

    static HEADER: HeaderValue = HeaderValue::from_static("");

    #[tokio::test]
    async fn mocked_api_get_account_holds() {
        let respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone2.expect_text().return_once(|| {
            Ok(r#"[
        {
            "id": "id",
            "created_at": "2014-11-06T10:34:47.123456Z",
            "amount": "string?",
            "ref": "String",
            "type": "hold_type"
        }
    ]"#
            .to_string())
        });
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        let output = api.get_account_holds(account, "id").await.unwrap();

        let acct = output;
        assert_eq!(acct.len(), 1);
    }

    #[tokio::test]
    async fn mocked_api_get_account_ledger() {
        let respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        // todo!(LedgerDetail Variants)
        respone2.expect_text().return_once(|| {
            Ok(r#"[
        {
            "id": "String",
            "amount": "0.1",
            "created_at": "2014-11-06T10:34:47.123456Z",
            "balance": "0.1",
            "type": "match",
            "details": {
                "order_id": "id strings",
                "product_id": "id strings",
                "trade_id": "id strings"
            }
        },
        {
            "id": "String",
            "amount": "0.1",
            "created_at": "2014-11-06T10:34:47.123456Z",
            "balance": "0.1",
            "type": "transfer",
            "details": {
                "transfer_id": "id strings",
                "transfer_type": "id strings"
            }
        },
        {
            "id": "String",
            "amount": "0.1",
            "created_at": "2014-11-06T10:34:47.123456Z",
            "balance": "0.1",
            "type": "fee",
            "details": {
                "transfer_id": "id strings",
                "transfer_type": "id strings"
            }
        },
        {
            "id": "String",
            "amount": "0.1",
            "created_at": "2014-11-06T10:34:47.123456Z",
            "balance": "0.1",
            "type": "rebate",
            "details": {
                "transfer_id": "id strings",
                "transfer_type": "id strings"
            }
        },
        {
            "id": "String",
            "amount": "0.1",
            "created_at": "2014-11-06T10:34:47.123456Z",
            "balance": "0.1",
            "type": "conversion",
            "details": {
                "transfer_id": "id strings",
                "transfer_type": "id strings"
            }
        }
    ]"#
            .to_string())
        });
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        let output = api.get_account_ledger(account, "id").await.unwrap();

        let acct = output;
        assert_eq!(acct.len(), 5);
        assert_eq!(acct[0].details.variant_name(), "match");
        assert_eq!(acct[1].details.variant_name(), "transfer");
        assert_eq!(acct[2].details.variant_name(), "fee");
        assert_eq!(acct[3].details.variant_name(), "rebate");
        assert_eq!(acct[4].details.variant_name(), "conversion");
    }

    #[tokio::test]
    async fn mocked_api_get_account_transfers() {
        let respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone2.expect_text().return_once(|| {
            Ok(r#"[
        {
            "id":"deadbeef-dead-beef-dead-beefdeadbeef",
            "type":"withdraw",
            "created_at":"2021-09-13 00:00:00.000000+00",
            "completed_at":"2021-09-13 00:00:00.000000+00",
            "canceled_at":null,
            "processed_at":"2021-09-13 00:00:00.00000+00",
            "user_nonce":"1234567891011",
            "amount":"0.00000000",
            "details":
            {
                    "fee":"0.000000",
                    "subtotal":"0.00",
                    "sent_to_address":"0xDEADBEEFDEADBEEFDEADBEEFDEADBEEFDEADBEEF",
                    "coinbase_account_id":"deadbeef-dead-beef-dead-beefdeadbeef",
                    "coinbase_withdrawal_id":"bofadeeznutsdeadbeefligm",
                    "coinbase_transaction_id":"bofadeeznutsdeadbeefligm",
                    "crypto_transaction_hash":"bofadeeznutsdeadbeefligmaballzsugmamikehuntjennytaliabofadeeznut",
                    "coinbase_payment_method_id":""
            }
        },
        {
            "id":"deadbeef-dead-beef-dead-beefdeadbeef",
            "type":"deposit",
            "created_at":"2021-09-13 00:00:00.000000+00",
            "completed_at":"2021-09-13 00:00:00.000000+00",
            "canceled_at":null,
            "processed_at":"2021-09-13 00:00:00.00000+00",
            "user_nonce":"1234567891011",
            "amount":"0.00000000",
            "details":
            {
                    "crypto_address":"asdasd",
                    "destination_tag":"asdasd",
                    "destination_tag_name": "String",
                    "coinbase_account_id":"deadbeef-dead-beef-dead-beefdeadbeef",
                    "crypto_transaction_id": "transaction id",
                    "coinbase_transaction_id":"bofadeeznutsdeadbeefligm",
                    "crypto_transaction_hash":"bofadeeznutsdeadbeefligmaballzsugmamikehuntjennytaliabofadeeznut"
            }
        }
    ]"#
            .to_string())
        });
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        let output = api.get_account_transfers(account, "id").await.unwrap();

        let acct = output;
        assert_eq!(acct.len(), 2);
    }

    #[tokio::test]
    async fn mocked_api_get_all_wallets() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone1.expect_text().return_once(|| {
            Ok(r#"[{
    "id": "String",
    "name": "String",
    "balance": "String",
    "currency": "String",
    "type": "String",
    "primary": false,
    "active": false,
    "available_on_consumer": false,
    "hold_balance": "0.1",
    "hold_currency": "string"
            }
    ]"#
            .to_string())
        });
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        let output = api.get_all_wallets(account).await.unwrap();

        let acct = output;
        assert_eq!(acct.len(), 1);
    }

    #[tokio::test]
    async fn mocked_api_get_all_currencies() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone1.expect_text().return_once(|| {
            Ok(r#"[{
                "id": "String",
                "name": "String",
                "min_size": "0.1",
                "status": "String",
                "message": "String",
                "max_precision": "0.1",
                "convertible_to": ["asda"],
                "details": {}
            }
    ]"#
            .to_string())
        });
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api.get_currencies().await.unwrap();

        let acct = output;
        assert_eq!(acct.len(), 1);
    }

    #[tokio::test]
    async fn mocked_api_get_all_currency() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone1.expect_text().return_once(|| {
            Ok(r#"{
                "id": "String",
                "name": "String",
                "min_size": "0.1",
                "status": "String",
                "message": "String",
                "max_precision": "0.1",
                "convertible_to": ["asda"],
                "details": {}
            }
    "#
            .to_string())
        });
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        api.get_currency("mine".to_string()).await.unwrap();
    }

    #[tokio::test]
    async fn mocked_api_get_conversion_invalid() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone1
            .expect_text()
            .return_once(|| Ok(r#"{"my json": 123}"#.to_string()));
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api.get_conversion(account, "", "").await;
        assert!(output.is_err());
    }

    #[tokio::test]
    async fn mocked_api_get_fills_invalid() {
        let respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone2
            .expect_text()
            .return_once(|| Ok(r#"{"my json": 123}"#.to_string()));
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api
            .get_fills(
                account,
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
            )
            .await;
        assert!(output.is_err());
    }

    #[tokio::test]
    async fn mocked_api_create_order_invalid() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone1
            .expect_text()
            .return_once(|| Ok(r#"{"my json": 123}"#.to_string()));
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);
        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let order = MarketOrder::new("".to_string(), Side::BUY, MarketOrderValue::Size(0.1));

        let output = api.create_order(account, order).await;
        assert!(output.is_err());
    }

    #[tokio::test]
    async fn mocked_api_get_single_order_invalid() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone1
            .expect_text()
            .return_once(|| Ok(r#"{"my json": 123}"#.to_string()));
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);
        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api.get_single_order(account, "".to_string()).await;
        assert!(output.is_err());
    }

    #[tokio::test]
    async fn mocked_api_get_orders_invalid() {
        let respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();
        let mut headers = MockHeaderMap::new();

        let account = APIKeyData {
            key: base64::encode("API KEY"),
            secret: base64::encode("API Secret"),
            passphrase: "passphrase".to_string(),
        };

        headers.expect_get::<&str>().return_const(Some(&HEADER));

        respone2
            .expect_text()
            .return_once(|| Ok(r#"{"my json": 123}"#.to_string()));
        respone2.expect_headers().return_const(headers);

        let mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);
        let mock_client = MockClient::new_mock(mock_request_builder.clone());
        let api = CBProAPI::from_client(mock_client);

        let output = api
            .get_orders(account, Some("".to_string()), Some("".to_string()))
            .await;
        assert!(output.is_err());
    }

    #[tokio::test]
    async fn mocked_api_subscription_request() {
        let builder = SubscriptionBuilder::new()
            .subscribe_to_heartbeat("".to_string())
            .subscribe_to_status()
            .subscribe_to_ticker("".to_string())
            .subscribe_to_full("".to_string())
            .subscribe_to_snapshot("".to_string());

        let request = builder.build();

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains(r#""name":"heartbeat","product_ids":[""]"#));
        assert!(serialized.contains(r#""name":"ticker","product_ids":[""]"#));
        assert!(serialized.contains(r#""name":"level2","product_ids":[""]"#));
        assert!(serialized.contains(r#""name":"full","product_ids":[""]"#));
        assert!(serialized.contains(r#""name":"status""#));
    }

    #[tokio::test]
    async fn mocked_api_subscription_request_vecs() {
        let builder = SubscriptionBuilder::new()
            .subscribe_to_heartbeat_vec(&mut vec!["".to_string()])
            .subscribe_to_ticker_vec(&mut vec!["".to_string()])
            .subscribe_to_full_vec(&mut vec!["".to_string()])
            .subscribe_to_snapshot_vec(&mut vec!["".to_string()]);

        let request = builder.build();

        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains(r#""name":"heartbeat","product_ids":[""]"#));
        assert!(serialized.contains(r#""name":"ticker","product_ids":[""]"#));
        assert!(serialized.contains(r#""name":"level2","product_ids":[""]"#));
        assert!(serialized.contains(r#""name":"full","product_ids":[""]"#));
    }

    #[tokio::test]
    async fn mock_stream_tc1() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mocked_tcp = MockTcpStream::new();
        let mut mock_tls = MockTlsStream::new(&mocked_tcp);

        mock_tls.expect_poll_write(|buf| buf == vec![0, 0, 0].as_slice());
        let out = mock_tls.write(vec![0, 0, 0].as_slice()).await.unwrap();
        assert_eq!(out, 3);
    }

    #[tokio::test]
    async fn mock_stream_tc2() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mocked_tcp = MockTcpStream::new();
        let mut mock_tls = MockTlsStream::new(&mocked_tcp);

        let output = vec![0, 1, 1];
        mock_tls.expect_poll_read(output.clone());
        let mut out: [u8; 3] = [0; 3];
        mock_tls.read_exact(&mut out).await.unwrap();

        assert_eq!(out.as_slice(), output.as_slice());
    }

    // #[tokio::test]
    // async fn mock_stream_tc3() {
    //     let request_builder = MockRequestBuilder::new_mock(vec![]);
    //     let client = MockClient::new_mock(request_builder);
    //
    //     let mocked_tcp = MockTcpStream::new();
    //     let mut mock_tls = MockTlsStream::new(&mocked_tcp);
    //     let mock_connector = MockTlsConnector::new(&mock_tls);
    //     mock_tls.expect_poll_write(|buf| {
    //         let as_str = String::from_utf8_lossy(buf);
    //         as_str.contains("Upgrade: websocket")
    //             && as_str.contains("Connection: Upgrade")
    //             && as_str.contains("User-Agent:")
    //             && as_str.contains("Sec-WebSocket-Key:")
    //             && as_str.contains("Sec-WebSocket-Protocol: chat")
    //             && as_str.contains("Sec-WebSocket-Version: 13")
    //     });
    //
    //     mock_tls.expect_poll_write(|buf| {
    //         println!("{:?}", String::from_utf8_lossy(buf));
    //         true
    //     });
    //
    //     mock_tls.expect_poll_read("\r\n\r\n".as_bytes().to_vec());
    //
    //     let request = SubscriptionBuilder::new()
    //         .subscribe_to_heartbeat("ETH-USD".to_string())
    //         .build();
    //
    //     let mut api = CBProAPI::from_client(client);
    //     let output = api.subscribe_to_websocket(request).await;
    //
    //     if output.is_err() {
    //         println!("{}", output.unwrap_err());
    //     }
    //
    //     //println!("{:?}", output);
    //     assert!(false);
    //     //let test = |test1| Box::new((async |test1| todo!())(test1));
    // }
}

#[cfg(all(test, feature = "mock"))]
mod websocket_stream_tests {
    use log::LevelFilter;
    use reqwest::Url;
    use simple_logger::SimpleLogger;
    use std::sync::Arc;
    use tokio::io::{
        AsyncReadExt,
        AsyncWriteExt,
    };
    use tokio::sync::Mutex;

    use crate::api::{
        CBProAPI,
        SubscriptionBuilder,
    };
    use crate::datastructs::orders::Side;

    use crate::mocked::{
        MockClient,
        MockIOBuilder,
        MockRequestBuilder,
        MockStream,
    };
    use crate::order_book::{
        OrderBook,
        OrderBookEntry,
    };
    use crate::websocket_lite::{
        FrameParser,
        ParserState,
        WebsocketStreamConnector,
    };

    pub(crate) fn default_websocket_upgrade_resp() -> Vec<u8> {
        format!("{}{}{}{}{}{}{}{}{}{}",
            "HTTP/1.1 101 Switching Protocols\r\n",
            "Date: Sun, 10 Apr 2022 13:23:52 GMT\r\n",
            "Connection: upgrade\r\n",
            "Upgrade: websocket\r\n",
            "Sec-WebSocket-Accept: HSmrc0sMlYUkAGmm5OPpG2HaGWk=\r\n",
            "CF-Cache-Status: DYNAMIC\r\n",
            "Expect-CT: max-age=604800, report-uri=\"https://report-uri.cloudflare.com/cdn-cgi/beacon/expect-ct\"\r\n",
            "Server: cloudflare\r\n",
            "CF-RAY: 6f9bccaa4a8f02ed-MIA\r\n",
            "\r\n")
        .as_bytes().to_vec()
    }

    pub(crate) fn frame_fin_16_0mask_256() -> Vec<u8> {
        let mut return_val: Vec<u8> = vec![
            0b10001111, 0b11111110, 0b00000001, 0b00000000, 0x00, 0x00, 0x00, 0x00,
        ];
        return_val.append(&mut [0u8; 256].to_vec());
        return_val
    }

    pub(crate) fn frame_fin_16_0mask_256_long() -> Vec<u8> {
        let mut return_val: Vec<u8> = vec![
            0b10001111, 0b11111111, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000001, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
        ];
        return_val.append(&mut [0u8; 256].to_vec());
        return_val
    }

    pub(crate) fn frame_fin_16_0mask_0_long() -> Vec<u8> {
        let return_val: Vec<u8> = vec![
            0b10001111, 0b11111111, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
        ];
        return_val
    }

    pub(crate) fn frame_fin_16_0mask_0_medium() -> Vec<u8> {
        let return_val: Vec<u8> = vec![
            0b10001111, 0b11111110, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000,
        ];
        return_val
    }

    pub(crate) fn frame_fin_16_1_short() -> Vec<u8> {
        let return_val: Vec<u8> = vec![0b10001111, 0b00000001, 0x00];
        return_val
    }

    pub(crate) fn frame_fin_16_1_long() -> Vec<u8> {
        let return_val: Vec<u8> = vec![
            0b10001111, 0b01111111, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000001, 0x00,
        ];
        return_val
    }

    pub(crate) fn frame_fin_16_1_medium() -> Vec<u8> {
        let return_val: Vec<u8> = vec![0b10001111, 0b01111110, 0b00000000, 0b00000001, 0x00];
        return_val
    }

    #[allow(unused)]
    pub(crate) fn frame_0() -> Vec<u8> {
        vec![0x00, 0x00]
    }

    pub(crate) fn frame_fin_0() -> Vec<u8> {
        vec![0b10000000, 0x00]
    }

    #[tokio::test]
    async fn websocket_stream_tc1() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };
        let mut stream = MockStream::new(&[0, 1, 2, 3]);

        assert_eq!(stream.read_u8().await.unwrap(), 0);
        assert_eq!(stream.read_u8().await.unwrap(), 1);
        assert_eq!(stream.read_u8().await.unwrap(), 2);
        assert_eq!(stream.read_u8().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn websocket_stream_tc2() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_16_0mask_256();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(frame[8..], output);
    }

    #[tokio::test]
    async fn websocket_stream_tc3() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_0();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(output.len(), 0);
    }

    #[tokio::test]
    async fn websocket_stream_tc4() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_16_0mask_256_long();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(frame[14..], output);
    }

    #[tokio::test]
    async fn websocket_stream_tc5() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_16_0mask_0_long();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(0, output.len());
    }

    #[tokio::test]
    async fn websocket_stream_tc6() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_16_0mask_0_medium();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(frame[8..], output);
    }

    #[tokio::test]
    async fn websocket_stream_tc7() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_16_1_short();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(1, output.len());
    }

    #[tokio::test]
    async fn websocket_stream_tc8() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_16_1_medium();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(1, output.len());
    }

    #[tokio::test]
    async fn websocket_stream_tc9() {
        match SimpleLogger::new().with_level(LevelFilter::Trace).init() {
            Ok(_) => {}
            Err(_) => {}
        };

        let mut handshake_response = default_websocket_upgrade_resp();

        let frame: Vec<u8> = frame_fin_16_1_long();

        handshake_response.append(&mut frame.clone());
        let mock_stream = MockStream::new(&handshake_response);

        //let stream = default_tls_stream("https://ws-feed.exchange.coinbase.com/").await;

        let mut websocket = WebsocketStreamConnector::new_no_sec()
            .connect(
                mock_stream,
                &Url::parse("https://ws-feed.exchange.coinbase.com/").unwrap(),
            )
            .await
            .unwrap();

        let mut output = Vec::new();
        websocket.read_to_end(&mut output).await.unwrap();

        //let writes: Vec<u8> = stream.writes.lock().unwrap().clone().into_iter().collect();
        assert_eq!(1, output.len());
    }

    #[tokio::test]
    async fn websocket_stream_sec_key() {
        let expected_key = "x3JJHMbDL1EzLkh9GBhXDw==";
        let expected_accept = "HSmrc0sMlYUkAGmm5OPpG2HaGWk=";

        assert_eq!(
            &WebsocketStreamConnector::sec_accept(expected_key),
            expected_accept
        );
    }

    #[tokio::test]
    async fn parser_state_display() {
        format!("{}", ParserState::FinalAndOpcode);
        format!("{}", ParserState::LongLength(0));
        format!("{}", ParserState::Mask(0));
        format!("{}", ParserState::MaskAndLength);
        format!("{}", ParserState::MediumLength(0));
        format!("{}", ParserState::Payload(0));
    }

    #[tokio::test]
    async fn api_websocket_sub_tc1() {
        let stream = MockStream::new(&[]);
        let stream_builder = MockIOBuilder::new(&stream);
        let client = MockClient::new_mock(MockRequestBuilder::new_mock(vec![]));

        CBProAPI::from_client_and_io_builder(client, stream_builder.clone());
    }

    pub(crate) fn websocket_sub_response() -> Vec<u8> {
        "\
        {\
            \"type\": \"subscribe\",\
            \"channels\": [{ \"name\": \"heartbeat\", \"product_ids\": [\"ETH-EUR\"] }]\
        }\
        "
        .as_bytes()
        .to_vec()
    }

    pub(crate) fn websocket_heartbeat_message() -> Vec<u8> {
        r#"{"type":"heartbeat","last_trade_id":278953096,"product_id":"ETH-USD","sequence":29716844241,"time":"2022-05-20T21:22:34.751219Z"}"#
        .as_bytes()
        .to_vec()
    }

    #[tokio::test]
    async fn api_websocket_sub_tc2() {
        let test_resp = websocket_heartbeat_message();
        let sub_resp = websocket_sub_response();
        let mut stream = MockStream::new(&sub_resp);
        stream.append_response(&test_resp).await;

        let stream_builder = MockIOBuilder::new(&stream);
        let client = MockClient::new_mock(MockRequestBuilder::new_mock(vec![]));

        let mut api = CBProAPI::from_client_and_io_builder(client, stream_builder.clone());

        api.subscribe_to_websocket(
            SubscriptionBuilder::new()
                .subscribe_to_full("my_product".to_string())
                .build(),
        )
        .await
        .unwrap();

        let writes = stream
            .writes
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>();
        assert_eq!(String::from_utf8(writes).unwrap(), "{\"type\":\"subscribe\",\"channels\":[{\"name\":\"full\",\"product_ids\":[\"my_product\"]}]}");

        api.read_websocket().await.unwrap();
    }

    pub(crate) fn websocket_status_message() -> Vec<u8> {
        r#"{"type":"status","currencies":[{"id":"EUR","name":"Euro","min_size":"0.01","status":"online","funding_account_id":"dfa7a9b4-3c16-4f79-ae1d-8d0292537ded","status_message":"","max_precision":"0.01","convertible_to":[],"details":{"type":"fiat","symbol":"€","network_confirmations":0,"sort_order":2,"crypto_address_link":"","crypto_transaction_link":"","push_payment_methods":["sepa_bank_account"],"group_types":["fiat","eur"]}},{"id":"RNDR","name":"Render Token","min_size":"0.00000001","status":"online","funding_account_id":"","status_message":"","max_precision":"0.00000001","convertible_to":[],"details":{"type":"crypto","symbol":"","network_confirmations":14,"sort_order":500,"crypto_address_link":"https://etherscan.io/token/0x6de037ef9ad2725eb40118bb1702ebb27e4aeb24?a={{address}}","crypto_transaction_link":"https://etherscan.io/tx/0x{{txId}}","push_payment_methods":["crypto"],"min_withdrawal_amount":1e-08,"max_withdrawal_amount":180000}}],"products":[{"id":"MINA-USDT","base_currency":"MINA","quote_currency":"USDT","base_min_size":"0.1","base_max_size":"160000","base_increment":"0.001","quote_increment":"0.001","display_name":"MINA/USDT","status":"online","margin_enabled":false,"status_message":"","min_market_funds":"1","max_market_funds":"250000","post_only":false,"limit_only":true,"cancel_only":false,"auction_mode":false,"type":"spot","fx_stablecoin":false,"max_slippage_percentage":"0.03000000"}]}"#.as_bytes().to_vec()
    }

    #[tokio::test]
    async fn api_websocket_status() {
        let test_resp = websocket_status_message();
        let sub_resp = websocket_sub_response();
        let mut stream = MockStream::new(&sub_resp);
        stream.append_response(&test_resp).await;

        let stream_builder = MockIOBuilder::new(&stream);
        let client = MockClient::new_mock(MockRequestBuilder::new_mock(vec![]));

        let mut api = CBProAPI::from_client_and_io_builder(client, stream_builder.clone());

        api.subscribe_to_websocket(
            SubscriptionBuilder::new()
                .subscribe_to_full("my_product".to_string())
                .build(),
        )
        .await
        .unwrap();

        let writes = stream
            .writes
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>();
        assert_eq!(String::from_utf8(writes).unwrap(), "{\"type\":\"subscribe\",\"channels\":[{\"name\":\"full\",\"product_ids\":[\"my_product\"]}]}");

        api.read_websocket().await.unwrap();
    }

    pub(crate) fn websocket_ticker_message() -> Vec<u8> {
        r#"{"type":"ticker","sequence":29892364716,"product_id":"ETH-USD","price":"1958.18","open_24h":"1930","volume_24h":"163658.03343361","low_24h":"1909.51","high_24h":"2020","volume_30d":"6409735.37447281","best_bid":"1958.18","best_ask":"1958.37","side":"sell","time":"2022-05-25T13:06:56.076339Z","trade_id":280884307,"last_size":"0.001"}"#.as_bytes().to_vec()
    }

    #[tokio::test]
    async fn api_websocket_ticker() {
        let test_resp = websocket_ticker_message();
        let sub_resp = websocket_sub_response();
        let mut stream = MockStream::new(&sub_resp);
        stream.append_response(&test_resp).await;

        let stream_builder = MockIOBuilder::new(&stream);
        let client = MockClient::new_mock(MockRequestBuilder::new_mock(vec![]));

        let mut api = CBProAPI::from_client_and_io_builder(client, stream_builder.clone());

        api.subscribe_to_websocket(
            SubscriptionBuilder::new()
                .subscribe_to_full("my_product".to_string())
                .build(),
        )
        .await
        .unwrap();

        let writes = stream
            .writes
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>();
        assert_eq!(String::from_utf8(writes).unwrap(), "{\"type\":\"subscribe\",\"channels\":[{\"name\":\"full\",\"product_ids\":[\"my_product\"]}]}");

        api.read_websocket().await.unwrap();
    }

    pub(crate) fn websocket_l2update_message() -> Vec<u8> {
        r#"{"type":"l2update","product_id":"ETH-USD","changes":[["sell","1957.66","0.00000000"]],"time":"2022-05-25T13:10:32.186903Z"}"#.as_bytes().to_vec()
    }

    pub(crate) fn websocket_l2snapshot_message() -> Vec<u8> {
        r#"{"type":"snapshot","product_id":"ETH-USD","asks":[["1955.81","0.00130538"],["1955.87","0.00000075"],["1955.99","0.22090846"],["1956.00","0.18126838"],["1956.02","2.10000000"],["1956.19","0.07814036"],["1956.21","0.10204759"],["1956.22","0.52470044"],["1956.23","8.00000000"],["1956.28","0.80982895"],["1956.29","0.14812073"]],"bids":[["1955.80","0.19478896"],["1955.79","0.79800000"],["1955.78","1.33910752"],["1955.77","2.55350878"],["1955.66","1.07737484"],["1955.62","2.29221645"]]}"#.as_bytes().to_vec()
    }

    #[tokio::test]
    async fn api_websocket_snapshot() {
        let sub_resp = websocket_sub_response();
        let mut stream = MockStream::new(&sub_resp);
        stream
            .append_response(&websocket_l2snapshot_message())
            .await;
        stream.append_response(&websocket_l2update_message()).await;

        let stream_builder = MockIOBuilder::new(&stream);
        let client = MockClient::new_mock(MockRequestBuilder::new_mock(vec![]));

        let mut api = CBProAPI::from_client_and_io_builder(client, stream_builder.clone());

        api.subscribe_to_websocket(
            SubscriptionBuilder::new()
                .subscribe_to_snapshot("my_product".to_string())
                .build(),
        )
        .await
        .unwrap();

        let writes = stream
            .writes
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>();
        assert_eq!(
            String::from_utf8(writes).unwrap(),
            r#"{"type":"subscribe","channels":[{"name":"level2","product_ids":["my_product"]}]}"#
        );

        api.read_websocket().await.unwrap();
    }

    pub(crate) fn websocket_l3limit_message() -> Vec<u8> {
        r#"{"order_id":"145110f7-362c-48d2-a7d0-a407918775dd","order_type":"limit","size":"1.06548906","price":"1950.34","client_oid":"8cb2eea3-4fa4-495b-a949-976940c4b021","type":"received","side":"sell","product_id":"ETH-USD","time":"2022-05-25T13:29:58.006556Z","sequence":29892914014}"#.as_bytes().to_vec()
    }

    pub(crate) fn websocket_l3cancel_message() -> Vec<u8> {
        r#"{"order_id":"dfbea570-d38b-4a08-ad0d-873e3ea73a0d","reason":"canceled","price":"1948.63","remaining_size":"0.44704","type":"done","side":"sell","product_id":"ETH-USD","time":"2022-05-25T13:29:57.988607Z","sequence":29892914013}"#.as_bytes().to_vec()
    }

    pub(crate) fn websocket_l3match_message() -> Vec<u8> {
        r#"{"price":"1948.35","order_id":"dbeb625b-42cb-4559-af17-225b96aa674c","remaining_size":"1.8","type":"open","side":"buy","product_id":"ETH-USD","time":"2022-05-25T13:29:57.980958Z","sequence":29892914009}"#.as_bytes().to_vec()
    }

    #[tokio::test]
    async fn api_websocket_l3() {
        let sub_resp = websocket_sub_response();
        let mut stream = MockStream::new(&sub_resp);
        stream.append_response(&websocket_l3limit_message()).await;
        stream.append_response(&websocket_l3match_message()).await;
        stream.append_response(&websocket_l3cancel_message()).await;

        let stream_builder = MockIOBuilder::new(&stream);
        let client = MockClient::new_mock(MockRequestBuilder::new_mock(vec![]));

        let mut api = CBProAPI::from_client_and_io_builder(client, stream_builder.clone());

        api.subscribe_to_websocket(
            SubscriptionBuilder::new()
                .subscribe_to_full("my_product".to_string())
                .build(),
        )
        .await
        .unwrap();

        let writes = stream
            .writes
            .lock()
            .unwrap()
            .clone()
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>();
        assert_eq!(
            String::from_utf8(writes).unwrap(),
            r#"{"type":"subscribe","channels":[{"name":"full","product_ids":["my_product"]}]}"#
        );

        api.read_websocket().await.unwrap();
        api.read_websocket().await.unwrap();
        api.read_websocket().await.unwrap();
    }

    #[tokio::test]
    async fn websocket_stream_test() {
        let mut mock_stream = MockStream::new(&default_websocket_upgrade_resp());
        mock_stream.append_response("response 2".as_bytes()).await;
        let websocket_connector = WebsocketStreamConnector::new_no_sec();

        let url = Url::parse("https://www.coinbase.com").unwrap();
        let mut stream = websocket_connector
            .connect(mock_stream.clone(), &url)
            .await
            .unwrap();
        stream.write_all("test write".as_bytes()).await.unwrap();

        let mut parser = FrameParser::default();

        let written = mock_stream.writes.lock().unwrap();

        for byte in &written[1] {
            match parser.process_byte(&byte).unwrap() {
                None => {}
                Some(frame) => {
                    dbg!(String::from_utf8_lossy(&frame.payload));
                }
            }
        }
    }

    #[tokio::test]
    async fn order_book_test_1() {
        let api = CBProAPI::default();
    }

    #[tokio::test]
    async fn order_book_test_2() {
        let mut order_book = OrderBook {
            bids: Arc::new(Mutex::new(vec![
                (0f64, 1f64).try_into().unwrap(),
                (1f64, 1f64).try_into().unwrap(),
                (2f64, 1f64).try_into().unwrap(),
            ])),
            asks: Arc::new(Mutex::new(vec![
                (6f64, 1f64).try_into().unwrap(),
                (5f64, 1f64).try_into().unwrap(),
                (4f64, 1f64).try_into().unwrap(),
            ])),
        };

        let ask_lock = order_book.asks.lock().await;
        let idx = OrderBook::find_ask_index(&ask_lock, &(5f64, 1f64).try_into().unwrap());
        assert_eq!(idx, 1);
        let idx = OrderBook::find_ask_index(&ask_lock, &(4f64, 1f64).try_into().unwrap());
        assert_eq!(idx, 2);
        let idx = OrderBook::find_ask_index(&ask_lock, &(3f64, 1f64).try_into().unwrap());
        assert_eq!(idx, 3);

        let bid_lock = order_book.bids.lock().await;
        let idx = OrderBook::find_bid_index(&bid_lock, &(1f64, 1f64).try_into().unwrap());
        assert_eq!(idx, 1);
        let idx = OrderBook::find_bid_index(&bid_lock, &(0f64, 1f64).try_into().unwrap());
        assert_eq!(idx, 0);
        let idx = OrderBook::find_bid_index(&bid_lock, &(3f64, 1f64).try_into().unwrap());
        assert_eq!(idx, 3);
    }

    #[tokio::test]
    async fn order_book_test_3() {
        let mut order_book = OrderBook {
            bids: Arc::new(Mutex::new(vec![
                (0f64, 1f64).try_into().unwrap(),
                (1f64, 1f64).try_into().unwrap(),
                (2f64, 1f64).try_into().unwrap(),
            ])),
            asks: Arc::new(Mutex::new(vec![
                (6f64, 1f64).try_into().unwrap(),
                (5f64, 1f64).try_into().unwrap(),
                (4f64, 1f64).try_into().unwrap(),
            ])),
        };

        order_book
            .apply_change(Side::BUY, (0f64, 0f64).try_into().unwrap())
            .await;
        assert_eq!(order_book.bids.lock().await.len(), 2);

        order_book
            .apply_change(Side::BUY, (1f64, 10f64).try_into().unwrap())
            .await;
        assert_eq!(order_book.bids.lock().await.len(), 2);
        let size: f64 = order_book.bids.lock().await[0].size.clone().into();
        assert_eq!(size, 10f64);

        order_book
            .apply_change(Side::BUY, (0f64, 11f64).try_into().unwrap())
            .await;
        assert_eq!(order_book.bids.lock().await.len(), 3);
        let size: f64 = order_book.bids.lock().await[0].size.clone().into();
        assert_eq!(size, 11f64);

        order_book
            .apply_change(Side::BUY, (0f64, 0f64).try_into().unwrap())
            .await;
        order_book
            .apply_change(Side::BUY, (1f64, 0f64).try_into().unwrap())
            .await;
        order_book
            .apply_change(Side::BUY, (2f64, 0f64).try_into().unwrap())
            .await;
        assert_eq!(order_book.bids.lock().await.len(), 0);

        order_book
            .apply_change(Side::BUY, (2f64, 0f64).try_into().unwrap())
            .await;
        assert_eq!(order_book.bids.lock().await.len(), 1);
    }
}

#[cfg(all(test, not(feature = "mock")))]
mod live_tests {
    use chrono::{
        DateTime,
        Utc,
    };
    use log::LevelFilter;
    use simple_logger::SimpleLogger;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    use crate::api::{
        CBProAPI,
        SubscriptionBuilder,
    };
    use crate::datastructs::websocket::WebsocketMessage;

    #[tokio::test]
    pub async fn heartbeat() {
        let res = SimpleLogger::new().with_level(LevelFilter::Debug).init();
        let mut api = CBProAPI::default();
        let request = SubscriptionBuilder::new()
            .subscribe_to_heartbeat("ETH-USD".to_string())
            .build();

        api.subscribe_to_websocket(request).await.unwrap();

        api.read_websocket().await.unwrap();
    }

    #[tokio::test]
    pub async fn status() {
        let res = SimpleLogger::new().with_level(LevelFilter::Debug).init();
        let mut api = CBProAPI::default();
        let request = SubscriptionBuilder::new().subscribe_to_status().build();

        api.subscribe_to_websocket(request).await.unwrap();

        api.read_websocket().await.unwrap();
    }

    #[tokio::test]
    pub async fn ticker() {
        let res = SimpleLogger::new().with_level(LevelFilter::Debug).init();
        let mut api = CBProAPI::default();
        let request = SubscriptionBuilder::new()
            .subscribe_to_ticker("ETH-USD".to_string())
            .build();

        api.subscribe_to_websocket(request).await.unwrap();

        api.read_websocket().await.unwrap();
        tokio::time::sleep(Duration::new(10, 0)).await;
        api.read_websocket().await.unwrap();
        api.read_websocket().await.unwrap();
        for x in 1..20 {
            api.read_websocket().await.unwrap();
        }
    }

    #[tokio::test]
    pub async fn level2() {
        let res = SimpleLogger::new().with_level(LevelFilter::Debug).init();
        let mut api = CBProAPI::default();
        let request = SubscriptionBuilder::new()
            .subscribe_to_snapshot("ETH-USD".to_string())
            .build();

        api.subscribe_to_websocket(request).await.unwrap();

        api.read_websocket().await.unwrap();
        tokio::time::sleep(Duration::new(10, 0)).await;
        api.read_websocket().await.unwrap();
        api.read_websocket().await.unwrap();
        for x in 1..20 {
            api.read_websocket().await.unwrap();
        }
    }

    #[tokio::test]
    pub async fn level3() {
        let res = SimpleLogger::new().with_level(LevelFilter::Debug).init();
        let mut api = CBProAPI::default();
        let request = SubscriptionBuilder::new()
            .subscribe_to_full("ETH-USD".to_string())
            .build();

        api.subscribe_to_websocket(request).await.unwrap();

        let mut rec = 0;
        let mut open = 0;
        let mut match_msg = 0;
        let mut done = 0;
        let mut change = 0;
        let mut activate = 0;

        for i in 0..10000 {
            match api.read_websocket().await.unwrap() {
                WebsocketMessage::Received(_) => {
                    rec += 1;
                    println!("Receive: {}", rec);
                }
                WebsocketMessage::Open(_) => {
                    open += 1;
                    println!("Open: {}", open);
                }
                WebsocketMessage::Match(_) => {
                    match_msg += 1;
                    println!("Match: {}", match_msg);
                }
                WebsocketMessage::Done(_) => {
                    done += 1;
                    println!("Done: {}", done);
                }
                WebsocketMessage::Change(_) => {
                    change += 1;
                    println!("Change: {}", change);
                }
                WebsocketMessage::Activate(_) => {
                    activate += 1;
                    println!("Activate: {}", activate);
                }
                _ => {}
            }
        }

        println!(
            "rec: {}, {}, {}, {}, {}, {}",
            rec, open, match_msg, done, change, activate
        );
    }
}
