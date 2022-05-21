//! coinbase_pro is an api for getting market data from the coinbase pro public API.
//! This crate aims to be a simple lightweight interface for making requests to coinbase's API.
//! This crate also aims to make available abstractions at the lowest level possible.
//! This allows users to specify how the responses get parsed.
//!
//! # Quickstart Info
//!
//! This api has a main client struct called [CBProAPI]. This struct is like a reqwest struct and
//! can be cheaply copied, cloned, and passed between threads. Internally it implements its
//! state utilizing [std::sync::Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html)
//! and [tokio::sync::Mutex](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html).
//!
//!
//! # Future Proofing
//!
//! In addition to the standard usage of this api through [CBProAPI], this crate exposes a low level
//! [CBRequestBuilder] that allows additional endpoints and custom deserialization if coinbase
//! ever changed their api, endpoints, or data formats.
//!
//!
//! # Examples
//!
//! ## Basic Usage
//! ```
//! use coinbase_pro::api::CBProAPI;
//!
//! fn main() {
//!     let api = CBProAPI::default();
//!     let product = api.get_product("ETH-USD".to_string()).await.unwrap();
//!
//!     assert_eq!(product.display_name, "ETH-USD");
//! }
//! ```
#![feature(backtrace)]
#![feature(termination_trait_lib)]
#![feature(process_exitcode_placeholder)]
extern crate core;

pub mod api;
mod deserialization;
mod errors;
pub mod requests;
mod websocket_lite;

pub mod datastructs;
mod mocked;

#[cfg(all(test, feature = "mock"))]
mod tests {
    use std::borrow::Borrow;
    use std::net::{
        IpAddr,
        SocketAddr,
    };
    use std::ops::Deref;
    use std::process::{
        ExitCode,
        Termination,
    };
    use std::rc::Rc;

    use log::{
        set_logger,
        LevelFilter,
    };
    use mockall::Any;
    use reqwest::header::HeaderValue;
    use reqwest::{
        IntoUrl,
        Method,
    };
    use simple_logger::SimpleLogger;
    use tokio::io::{
        AsyncReadExt,
        AsyncWriteExt,
    };
    use tokio_test::assert_err;

    use crate::api::{
        APIKeyData,
        CBProAPI,
        Level,
        SubscriptionBuilder,
    };
    use crate::datastructs::accounts::LedgerDetail;
    use crate::datastructs::orders::{
        MarketOrder,
        MarketOrderValue,
        Order,
        Side,
    };
    use crate::datastructs::websocket::SubscribeRequest;
    use crate::errors::Error;
    use crate::errors::WebsocketError::FrameSize;
    use crate::mocked;
    use crate::mocked::{
        CallInfo,
        MockClient,
        MockHeaderMap,
        MockRequestBuilder,
        MockResponse,
        MockTcpStream,
        MockTlsConnector,
        MockTlsStream,
    };

    #[tokio::test]
    async fn mocked_api_coinbase_server_error() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();

        respone1.expect_text().return_once(|| {
            Ok(r#"
            {
                "message": "error message"
            }"#
            .to_string())
        });

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone2 = MockResponse::new();

        respone1
            .expect_text()
            .return_once(|| Ok(r#"[{"id":"Causes Error",}]"#.to_string()));

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

        let output = api.get_all_products().await;

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
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

        let output = api.get_all_products().await.unwrap();

        let prod = output.get(0).unwrap();

        assert_eq!(prod.trading_disabled, None);
        assert_eq!(prod.fx_stablecoin, Some(false));
        assert_eq!(prod.max_slippage_percentage, Some(0.00000001));
    }

    #[tokio::test]
    async fn mocked_api_get_product_minimum_required_response() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone2 = MockResponse::new();

        respone1
            .expect_text()
            .return_once(|| Ok(r#"{"id":"UMA-EUR"}"#.to_string()));

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client.clone());

        let output = api.get_product("MyProduct".to_string()).await;

        assert!(mock_client
            .requested_url
            .borrow_mut()
            .contains("/MyProduct"));
    }

    #[tokio::test]
    async fn mocked_api_get_product_book_minimum_response() {
        let mut respone1 = MockResponse::new();
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone2 = MockResponse::new();

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone1 = MockResponse::new();
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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone1 = MockResponse::new();
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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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
        let mut respone1 = MockResponse::new();
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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

        let output = api.get_currency("mine".to_string()).await.unwrap();

        let acct = output;
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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

        let output = api.get_conversion(account, "", "").await;
        assert!(output.is_err());
    }

    #[tokio::test]
    async fn mocked_api_get_fills_invalid() {
        let mut respone1 = MockResponse::new();
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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);

        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);
        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);
        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

        let output = api.get_single_order(account, "".to_string()).await;
        assert!(output.is_err());
    }

    #[tokio::test]
    async fn mocked_api_get_orders_invalid() {
        let mut respone1 = MockResponse::new();
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

        let mut mock_request_builder = MockRequestBuilder::new_mock(vec![respone1, respone2]);
        let mut mock_client = MockClient::new_mock(mock_request_builder.clone());
        let mut api = CBProAPI::from_client(mock_client);

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

    pub struct FrameInfo<'a> {
        fin: bool,
        opcode: u8,
        masked: bool,
        length: u64,
        mask: &'a [u8],
        payload: &'a [u8],
    }

    pub struct FrameTestCase<'a> {
        raw_data: &'a [u8],
        expected_info: FrameInfo<'a>,
    }

    #[tokio::test]
    async fn mock_stream_tc1() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let addr = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 307);
        let mocked_tcp = MockTcpStream::new();
        let mut mock_tls = MockTlsStream::new(&mocked_tcp);
        let mock_connector = MockTlsConnector::new(&mock_tls);

        mock_tls.expect_poll_write(|buf| buf == vec![0, 0, 0].as_slice());
        let out = mock_tls.write(vec![0, 0, 0].as_slice()).await.unwrap();
        assert_eq!(out, 3);
    }

    #[tokio::test]
    async fn mock_stream_tc2() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let addr = SocketAddr::new(IpAddr::from([127, 0, 0, 1]), 307);
        let mocked_tcp = MockTcpStream::new();
        let mut mock_tls = MockTlsStream::new(&mocked_tcp);
        let mock_connector = MockTlsConnector::new(&mock_tls);

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
    use log::{
        error,
        LevelFilter,
    };
    use reqwest::Url;
    use simple_logger::SimpleLogger;
    use tokio::io::{
        AsyncRead,
        AsyncReadExt,
    };
    use tokio::net::TcpStream;
    use tokio_native_tls::native_tls::TlsConnector as NativeTlsConnector;
    use tokio_native_tls::TlsConnector;

    use crate::api::{
        default_tls_stream,
        CBProAPI,
        SubscriptionBuilder,
    };
    use crate::datastructs::websocket::SubscribeRequest;
    use crate::mocked::{
        MockClient,
        MockIOBuilder,
        MockRequestBuilder,
        MockStream,
    };
    use crate::websocket_lite::{
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
        let mut return_val: Vec<u8> = vec![
            0b10001111, 0b11111111, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
        ];
        return_val
    }

    pub(crate) fn frame_fin_16_0mask_0_medium() -> Vec<u8> {
        let mut return_val: Vec<u8> = vec![
            0b10001111, 0b11111110, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000,
        ];
        return_val
    }

    pub(crate) fn frame_fin_16_1_short() -> Vec<u8> {
        let mut return_val: Vec<u8> = vec![0b10001111, 0b00000001, 0x00];
        return_val
    }

    pub(crate) fn frame_fin_16_1_long() -> Vec<u8> {
        let mut return_val: Vec<u8> = vec![
            0b10001111, 0b01111111, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
            0b00000000, 0b00000000, 0b00000001, 0x00,
        ];
        return_val
    }

    pub(crate) fn frame_fin_16_1_medium() -> Vec<u8> {
        let mut return_val: Vec<u8> = vec![0b10001111, 0b01111110, 0b00000000, 0b00000001, 0x00];
        return_val
    }

    pub(crate) fn frame_0() -> Vec<u8> {
        vec![0x00, 0x00]
    }

    pub(crate) fn frame_fin_0() -> Vec<u8> {
        vec![0b10000000, 0x00]
    }

    #[tokio::test]
    async fn websocket_stream_tc1() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();
        let mut stream = MockStream::new(&[0, 1, 2, 3]);

        assert_eq!(stream.read_u8().await.unwrap(), 0);
        assert_eq!(stream.read_u8().await.unwrap(), 1);
        assert_eq!(stream.read_u8().await.unwrap(), 2);
        assert_eq!(stream.read_u8().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn websocket_stream_tc2() {
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_16_0mask_256();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_0();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_16_0mask_256_long();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_16_0mask_0_long();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_16_0mask_0_medium();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_16_1_short();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_16_1_medium();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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
        SimpleLogger::new().with_level(LevelFilter::Trace).init();

        let mut handshake_response = default_websocket_upgrade_resp();

        let mut frame: Vec<u8> = frame_fin_16_1_long();

        handshake_response.append(&mut frame.clone());
        let mut mock_stream = MockStream::new(&handshake_response);

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

        let api = CBProAPI::from_client_and_io_builder(client, stream_builder.clone());
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
        "\
        {\"type\":\"heartbeat\",\"last_trade_id\":278953096,\"product_id\":\"ETH-USD\",\"sequence\":29716844241,\"time\":\"2022-05-20T21:22:34.751219Z\"}\
        "
        .as_bytes()
        .to_vec()
    }

    #[tokio::test]
    async fn api_websocket_sub_tc2() {
        let stream = MockStream::new(&websocket_sub_response());
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
            .collect::<Vec<u8>>();
        assert_eq!(String::from_utf8(writes).unwrap(), "{\"type\":\"subscribe\",\"channels\":[{\"name\":\"full\",\"product_ids\":[\"my_product\"]}]}");
    }
}

#[cfg(all(test, not(feature = "mock")))]
mod live_tests {
    use log::LevelFilter;
    use simple_logger::SimpleLogger;

    use crate::api::{
        CBProAPI,
        SubscriptionBuilder,
    };

    #[tokio::test]
    pub async fn heartbeat() {
        let res = SimpleLogger::new().with_level(LevelFilter::Debug).init();
        let mut api = CBProAPI::default();
        let request = SubscriptionBuilder::new()
            .subscribe_to_heartbeat("ETH-USD".to_string())
            .build();

        api.subscribe_to_websocket(request).await.unwrap();

        api.read_websocket().await.unwrap();
        api.read_websocket().await.unwrap();
        api.read_websocket().await.unwrap();
    }
}
