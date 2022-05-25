# Coinbase Pro API
![Build](https://img.shields.io/github/workflow/status/bikester1/coinbase_pro_rust_api/Rust/main?style=for-the-badge)
![coverage](https://img.shields.io/badge/Coverage-84%25-yellow?style=for-the-badge)


coinbase_pro is an api for getting market data from the coinbase pro public API.
This crate aims to be a simple lightweight interface for making requests to coinbase's API.
This crate also aims to make available abstractions at the lowest level possible.
This allows users to specify how the responses get parsed.

## Quickstart Info

This api has a main client struct called [CBProAPI]. This struct is like a reqwest struct and
can be cheaply copied, cloned, and passed between threads. Internally it implements its
state utilizing [std::sync::Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html)
and [tokio::sync::Mutex](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html).


## Future Proofing

In addition to the standard usage of this api through [CBProAPI], this crate exposes a low level
[CBRequestBuilder] that allows additional endpoints and custom deserialization if coinbase
ever changed their api, endpoints, or data formats.


## Examples

### Basic Usage
```
use coinbase_pro::api::CBProAPI;

#[tokio::test]
async fn get_product() {
    let api = CBProAPI::default();
    let product = api.get_product("ETH-USD".to_string()).await.unwrap();

    assert_eq!(product.display_name, "ETH-USD");
}
```

### Websocket
```
use coinbase_pro::api::CBProAPI;
use coinbase_pro::api::SubscriptionBuilder;

#[tokio::test]
async fn subscribe() {
    let mut api = CBProAPI::default();
    let subscribe_message = SubscriptionBuilder::new()
        .subscribe_to_heartbeat("ETH-USD".to_string())
        .build();

    api.subscribe_to_websocket(subscribe_message).await.unwrap();
    
    let response = api.read_websocket().await.unwrap();
}
```
