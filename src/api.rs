#[cfg(backtrace)]
use std::backtrace::Backtrace;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::io::Read;
use std::net::SocketAddr;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use log::debug;
use reqwest::Url;
use serde::{
    Deserialize,
    Serialize,
};
use tokio::io::{
    AsyncRead,
    AsyncReadExt,
    AsyncWrite,
    AsyncWriteExt,
};
use tokio::join;
use tokio::net::TcpStream;
use tokio::sync::{
    Mutex,
    MutexGuard,
};
use tokio::time::Instant;
use tokio_native_tls::native_tls::TlsConnector as NativeTlsConnector;
use tokio_native_tls::{
    TlsConnector,
    TlsStream,
};

#[cfg(not(feature = "mock"))]
use mocked::*;
#[cfg(feature = "mock")]
use mocks::*;

use crate::datastructs::accounts::{
    Account,
    Fees,
    Hold,
    Ledger,
    Transfer,
    Wallet,
};
use crate::datastructs::orders::{
    CoinbaseOrder,
    Fill,
    NewOrderResponse,
    Order,
};
use crate::datastructs::products::{
    Currency,
    Product,
    ProductBook,
};
use crate::errors::WebsocketError::{
    NoSocketAddressError,
    NoWebsocketConnectionError,
    SocketAddressError,
    TCPConnectionError,
    TLSConnectionError,
    URLParseError,
    WebsocketConnectionError,
    WebsocketIOError,
};
use crate::errors::{
    Error,
    SerdeJSONParseError,
    WebsocketError,
};
use crate::requests::{
    CBRequestBuilder,
    RequestMethod,
};
use crate::websocket_lite::{
    AsyncIO,
    WebsocketStreamConnector,
};

#[cfg(feature = "mock")]
mod mocks {
    pub use crate::mocked::{
        MockClient as Client,
        MockRequestBuilder as RequestBuilder,
    };
}
mod mocked {
    pub use reqwest::Client;
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// #[serde(untagged)]
// pub enum CBProResponse<T> {
//     Data(T),
//     Error(CBProServerError),
// }

///id: string required
/// amount: string required
/// from_account_id: string required
/// to_account_id: string required
/// from: string required
/// to: string required
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Conversion {
    id: String,
    amount: String,
    from_account_id: String,
    to_account_id: String,
    from: String,
    to: String,
}

/// Use a subscription builder to create new subscriptions to a coinbase websocket.<br>
/// Subscriptions can be chained with [Self::build()] at the end to return the final subscription request.<br>
///
/// ## Example
///```
/// use coinbase_pro::api::SubscriptionBuilder;
/// let subscription = SubscriptionBuilder::new()
///    .subscribe_to_ticker("ETH-USD".to_string())
///    .subscribe_to_status()
///    .subscribe_to_heartbeat("BTC-USD".to_string())
///    .subscribe_to_heartbeat("ETH-USD".to_string())
///    .build();
///```
pub struct SubscriptionBuilder {
    heartbeat_products: Vec<String>,
    ticker_products: Vec<String>,
    l2_products: Vec<String>,
    match_products: Vec<String>,
    full_channel_products: Vec<String>,
    status_channel: bool,
}

impl SubscriptionBuilder {
    pub fn new() -> Self {
        SubscriptionBuilder {
            heartbeat_products: vec![],
            ticker_products: vec![],
            l2_products: vec![],
            match_products: vec![],
            full_channel_products: vec![],
            status_channel: false,
        }
    }

    /// Include a heartbeat subscription for specified product.<br>
    /// Todo: Add checks to prevent the same products from being added.
    /// ## Example
    ///```
    /// use coinbase_pro::api::SubscriptionBuilder;
    /// let subscription = SubscriptionBuilder::new()
    ///    .subscribe_to_heartbeat("BTC-USD".to_string())
    ///    .subscribe_to_heartbeat("ETH-USD".to_string())
    ///    .build();
    ///```
    pub fn subscribe_to_heartbeat(mut self, product: String) -> Self {
        self.heartbeat_products.push(product);
        self
    }

    /// Include a heartbeat subscription for a [Vec<String>] of products.<br>
    /// Todo: Add checks to prevent the same products from being added.
    /// ## Example
    ///```
    /// use coinbase_pro::api::SubscriptionBuilder;
    /// let subscription = SubscriptionBuilder::new()
    ///    .subscribe_to_heartbeat_vec(&mut vec!["BTC-USD".to_string(), "ETH-USD".to_string()])
    ///    .build();
    ///```
    pub fn subscribe_to_heartbeat_vec(mut self, product: &mut Vec<String>) -> Self {
        self.heartbeat_products.append(product);
        self
    }

    pub fn subscribe_to_status(mut self) -> Self {
        self.status_channel = true;
        self
    }

    pub fn subscribe_to_ticker(mut self, product: String) -> Self {
        self.ticker_products.push(product);
        self
    }

    pub fn subscribe_to_ticker_vec(mut self, product: &mut Vec<String>) -> Self {
        self.ticker_products.append(product);
        self
    }

    pub fn subscribe_to_snapshot(mut self, product: String) -> Self {
        self.l2_products.push(product);
        self
    }

    pub fn subscribe_to_snapshot_vec(mut self, product: &mut Vec<String>) -> Self {
        self.l2_products.append(product);
        self
    }

    pub fn subscribe_to_full(mut self, product: String) -> Self {
        self.full_channel_products.push(product);
        self
    }

    pub fn subscribe_to_full_vec(mut self, product: &mut Vec<String>) -> Self {
        self.full_channel_products.append(product);
        self
    }

    /// Finalize request and return the struct used to serialize into a websocket payload consuming the builder in the process.<br>
    pub fn build(self) -> crate::datastructs::websocket::SubscribeRequest {
        let mut request = crate::datastructs::websocket::SubscribeRequest { channels: vec![] };

        if self.heartbeat_products.len() > 0 {
            request
                .channels
                .push(crate::datastructs::websocket::Channel::Heartbeat(
                    crate::datastructs::websocket::HeartbeatChannel {
                        product_ids: self.heartbeat_products,
                    },
                ));
        }

        if self.ticker_products.len() > 0 {
            request
                .channels
                .push(crate::datastructs::websocket::Channel::Ticker(
                    crate::datastructs::websocket::TickerChannel {
                        product_ids: self.ticker_products,
                    },
                ));
        }

        if self.l2_products.len() > 0 {
            request
                .channels
                .push(crate::datastructs::websocket::Channel::Level2(
                    crate::datastructs::websocket::Level2Channel {
                        product_ids: self.l2_products,
                    },
                ));
        }

        if self.full_channel_products.len() > 0 {
            request
                .channels
                .push(crate::datastructs::websocket::Channel::Full(
                    crate::datastructs::websocket::FullChannel {
                        product_ids: self.full_channel_products,
                    },
                ));
        }

        if self.status_channel {
            request
                .channels
                .push(crate::datastructs::websocket::Channel::Status(
                    crate::datastructs::websocket::StatusChannel {},
                ));
        }

        request
    }
}

#[derive(Clone)]
pub enum Level {
    One = 1,
    Two = 2,
    Three = 3,
}

impl Level {
    pub fn as_string(&self) -> String {
        (self.clone() as u8).to_string()
    }
}

#[derive(Clone)]
pub(crate) struct RateLimitedPool {
    last_req: Arc<Mutex<Instant>>,
    rate_limit: Arc<f32>,
}

impl RateLimitedPool {
    pub fn new(rate_per_second: f32) -> Self {
        RateLimitedPool {
            last_req: Arc::new(Mutex::new(Instant::now())),
            rate_limit: Arc::new(rate_per_second),
        }
    }

    fn rate_limit_time_delta_micros(self: &Self) -> u64 {
        (1000000.0 / self.rate_limit.deref()) as u64
    }

    pub async fn schedule_rate_limited_task<O>(self: &Self, fut: impl Future<Output = O>) -> O {
        self.wait_in_queue(fut).await
    }

    async fn wait_in_queue<O>(self: &Self, fut: impl Future<Output = O>) -> O {
        let mut fut_instant = tokio::time::sleep_until(Instant::now());
        let mut fut_last_req = self.last_req.deref().lock();

        loop {
            // wait turn and wait for lock. If we get a lock we still wait for the time and just take this req as the next one to execute.
            let (_, mut last_req) = join!(fut_instant, fut_last_req);

            let next_available_instant: Instant = *last_req.deref()
                + tokio::time::Duration::from_micros(self.rate_limit_time_delta_micros());

            // if we aren't ready to execute
            if Instant::now() < next_available_instant {
                // wait and keep looping
                fut_instant = tokio::time::sleep_until(next_available_instant);
                fut_last_req = self.last_req.deref().lock();
            } else {
                // set last req time and return.
                *last_req = Instant::now();
                break;
            }
        }

        fut.await
    }
}

#[async_trait]
pub trait AsyncIOBuilder {
    async fn new_stream(&self, url: &str) -> Result<Box<dyn AsyncIO>, WebsocketError>;
}

#[derive(Clone)]
pub struct TokioTlsStreamBuilder {}

#[async_trait]
impl AsyncIOBuilder for TokioTlsStreamBuilder {
    async fn new_stream(&self, url: &str) -> Result<Box<dyn AsyncIO>, WebsocketError> {
        let url = Url::parse(url).map_err(|err| WebsocketError::URLParseError {
            source: Box::new(err),
            url: url.to_string(),
        })?;

        let &address = url
            .socket_addrs(|| None)?
            .get(0)
            .ok_or(NoSocketAddressError {
                url: url.to_string(),
            })?;

        let tcp_stream = tokio::net::TcpStream::connect(address)
            .await
            .map_err(|err| TCPConnectionError {
                source: Box::new(err),
                url: url.to_string(),
            })?;

        let native_connector =
            tokio_native_tls::native_tls::TlsConnector::new().map_err(|err| {
                TLSConnectionError {
                    source: Box::new(err),
                    #[cfg(backtrace)]
                    backtrace: Backtrace::capture(),
                    url: url.to_string(),
                }
            })?;

        let connector = TlsConnector::from(native_connector);

        let tls_stream = connector
            .connect(url.domain().unwrap_or(""), tcp_stream)
            .await
            .map_err(|err| TLSConnectionError {
                source: Box::new(err),
                #[cfg(backtrace)]
                backtrace: Backtrace::capture(),
                url: url.to_string(),
            })?;

        let websocket = WebsocketStreamConnector::default()
            .connect(tls_stream, &url)
            .await
            .map_err(|err| WebsocketConnectionError {
                source: Box::new(err),
                url: url.to_string(),
            })?;

        Ok(Box::new(websocket))
    }
}

pub(crate) async fn default_tls_stream(url: &str) -> TlsStream<TcpStream> {
    let url = Url::parse(url).unwrap();
    let sock = url.socket_addrs(|| None).unwrap();
    let tcp = TcpStream::connect(sock[0]).await.unwrap();
    let connector = NativeTlsConnector::new().unwrap();
    let connector = TlsConnector::from(connector);
    connector.connect(url.domain().unwrap(), tcp).await.unwrap()
}

/// # Main crate interface
/// CBProAPI is the main entry point to CBPro data.
///
/// This type utilizes [Arc]s so there is no need to wrap in a pointer type.
/// This type also contains a [reqwest::Client] so it is advised to initalize one instance of the API and clone where it is needed.
#[derive(Clone)]
pub struct CBProAPI {
    pub client: Client,
    coin_base_url: Arc<Mutex<String>>,

    /// Rate limit in requests per second
    pool: RateLimitedPool,
    user_agent: Arc<String>,

    websocket_connector: Arc<Mutex<Box<dyn AsyncIOBuilder>>>,
    websocket: Arc<Mutex<Option<Box<dyn AsyncIO>>>>,
    wss_url: Arc<Mutex<String>>,
}

#[derive(Clone)]
pub struct APIKeyData {
    pub key: String,
    pub secret: String,
    pub passphrase: String,
}

/// Creates a CBProAPI instance with useful default values.
impl Default for CBProAPI {
    fn default() -> Self {
        CBProAPI {
            client: Client::new(),
            user_agent: Arc::new("Rust".to_string()),
            pool: RateLimitedPool::new(10.0),
            websocket: Arc::new(Mutex::new(None)),
            coin_base_url: Arc::new(Mutex::new("".to_string())),
            wss_url: Arc::new(Mutex::new(
                "https://ws-feed.exchange.coinbase.com/".to_string(),
            )),
            websocket_connector: Arc::new(Mutex::new(Box::new(TokioTlsStreamBuilder {}))),
        }
    }
}

impl CBProAPI {
    pub fn from_client(client: Client) -> Self {
        CBProAPI {
            client,
            user_agent: Arc::new("Rust".to_string()),
            pool: RateLimitedPool::new(10.0),
            websocket: Arc::new(Mutex::new(None)),
            coin_base_url: Arc::new(Mutex::new("".to_string())),
            wss_url: Arc::new(Mutex::new(
                "https://ws-feed.exchange.coinbase.com/".to_string(),
            )),
            websocket_connector: Arc::new(Mutex::new(Box::new(TokioTlsStreamBuilder {}))),
        }
    }

    pub fn from_client_and_io_builder(
        client: Client,
        builder: impl AsyncIOBuilder + 'static,
    ) -> Self {
        CBProAPI {
            client,
            user_agent: Arc::new("Rust".to_string()),
            pool: RateLimitedPool::new(10.0),
            websocket: Arc::new(Mutex::new(None)),
            coin_base_url: Arc::new(Mutex::new("".to_string())),
            wss_url: Arc::new(Mutex::new(
                "https://ws-feed.exchange.coinbase.com/".to_string(),
            )),
            websocket_connector: Arc::new(Mutex::new(Box::new(builder))),
        }
    }

    /// Send a [Get Product Book Request](https://docs.cloud.coinbase.com/exchange/reference/exchangerestapi_getproductbook) and return a result containing the requested product book or an error.
    pub async fn get_product_book(
        self: &Self,
        product_id: String,
        level: Option<Level>,
    ) -> Result<ProductBook, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .try_add_query_param("level".to_string(), level.map(|x| x.as_string()))
            .set_endpoint(format!("/products/{}/book", product_id))
            .exec::<ProductBook>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_all_products(self: &Self) -> Result<Vec<Product>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/products"))
            .exec::<Vec<Product>>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_product(self: &Self, product_id: String) -> Result<Product, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/products/{}", product_id))
            .exec::<Product>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_fees(self: &Self, account: APIKeyData) -> Result<Fees, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/fees"))
            .sign(account)
            .exec::<Fees>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_accounts(self: &Self, account: APIKeyData) -> Result<Vec<Account>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/accounts"))
            .sign(account)
            .exec::<Vec<Account>>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_account(
        self: &Self,
        account: APIKeyData,
        account_id: &str,
    ) -> Result<Account, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/accounts/{}", account_id))
            .sign(account)
            .exec::<Account>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_account_holds(
        self: &Self,
        account: APIKeyData,
        account_id: &str,
    ) -> Result<Vec<Hold>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/accounts/{}/holds", account_id))
            .sign(account)
            .exec_pagenated::<Hold>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_account_ledger(
        self: &Self,
        account: APIKeyData,
        account_id: &str,
    ) -> Result<Vec<Ledger>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/accounts/{}/ledger", account_id))
            .sign(account)
            .exec_pagenated::<Ledger>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_all_wallets(self: &Self, account: APIKeyData) -> Result<Vec<Wallet>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/coinbase-accounts"))
            .sign(account)
            .exec::<Vec<Wallet>>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_account_transfers(
        self: &Self,
        account: APIKeyData,
        account_id: &str,
    ) -> Result<Vec<Transfer>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/accounts/{}/transfers", account_id))
            .sign(account)
            .exec_pagenated::<Transfer>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_conversion(
        self: &Self,
        account: APIKeyData,
        conversion_id: &str,
        profile_id: &str,
    ) -> Result<Conversion, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/conversions/{}", conversion_id))
            .add_query_param("profile_id".to_string(), profile_id.to_string())
            .sign(account)
            .exec::<Conversion>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_currencies(self: &Self) -> Result<Vec<Currency>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/currencies"))
            .exec::<Vec<Currency>>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_currency(self: &Self, currency_id: String) -> Result<Currency, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/currencies/{}", currency_id))
            .exec::<Currency>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_fills(
        self: &Self,
        account: APIKeyData,
        order_id: Option<String>,
        product_id: Option<String>,
        profile_id: Option<String>,
    ) -> Result<Vec<Fill>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/fills"))
            .try_add_query_param("order_id".to_string(), order_id)
            .try_add_query_param("product_id".to_string(), product_id)
            .try_add_query_param("profile_id".to_string(), profile_id)
            .sign(account)
            .exec_pagenated::<Fill>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_orders(
        self: &Self,
        account: APIKeyData,
        product_id: Option<String>,
        profile_id: Option<String>,
    ) -> Result<Vec<Order>, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/orders"))
            .try_add_query_param("product_id".to_string(), product_id)
            .try_add_query_param("profile_id".to_string(), profile_id)
            .sign(account)
            .exec_pagenated::<Order>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn create_order(
        self: &Self,
        account: APIKeyData,
        order: impl Serialize + CoinbaseOrder,
    ) -> Result<NewOrderResponse, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/orders"))
            .set_method(RequestMethod::POST)
            .set_body(order)?
            .sign(account)
            .exec::<NewOrderResponse>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    pub async fn get_single_order(
        self: &Self,
        account: APIKeyData,
        order_id: String,
    ) -> Result<NewOrderResponse, Error> {
        let future = CBRequestBuilder::new(&self.client, self.user_agent.deref().clone())
            .set_endpoint(format!("/orders/{}", order_id))
            .sign(account)
            .exec::<NewOrderResponse>();

        self.pool.clone().schedule_rate_limited_task(future).await
    }

    /// performs a write and a read of the websocket.
    ///
    /// todo! add check to the received message to make sure the desired channels are subscribed.
    pub async fn subscribe_to_websocket(
        &mut self,
        subscription: crate::datastructs::websocket::SubscribeRequest,
    ) -> Result<crate::datastructs::websocket::WebsocketMessage, WebsocketError> {
        let init;
        {
            let websock = self.websocket.lock().await;
            init = websock.is_none();
        }
        if init {
            let new_websocket = self
                .websocket_connector
                .lock()
                .await
                .new_stream(self.wss_url.lock().await.clone().as_str())
                .await?;

            self.websocket = Arc::new(Mutex::new(Some(new_websocket)));
        }

        let mut lock = self.websocket.lock().await;
        lock.borrow_mut()
            .as_mut()
            .unwrap()
            .write_all(
                serde_json::to_string(&crate::datastructs::websocket::WebsocketMessage::Subscribe(
                    subscription.clone(),
                ))
                .map_err(|err| SerdeJSONParseError {
                    message: format!("{:?}", subscription),
                    source: err,
                })?
                .as_bytes(),
            )
            .await
            .map_err(|err| WebsocketIOError {
                source: Box::new(err),
                #[cfg(backtrace)]
                backtrace: Backtrace::capture(),
                context: Some(HashMap::from([(
                    "Subscription".to_string(),
                    format!("{:?}", subscription),
                )])),
            })?;

        Self::read_websocket_with_lock(lock).await
    }

    /// reads the websocket and returns the next Websocket message received
    pub async fn read_websocket(
        &mut self,
    ) -> Result<crate::datastructs::websocket::WebsocketMessage, WebsocketError> {
        let lock = self.websocket.lock().await;

        Self::read_websocket_with_lock(lock).await
    }

    /// reads the websocket into a websocket message given a MutexGuard to the WebsocketConnection.
    async fn read_websocket_with_lock(
        mut lock: MutexGuard<'_, Option<Box<dyn AsyncIO>>>,
    ) -> Result<crate::datastructs::websocket::WebsocketMessage, WebsocketError> {
        let websocket = lock
            .borrow_mut()
            .as_mut()
            .ok_or(NoWebsocketConnectionError)?;

        let mut buf = Vec::new();
        websocket
            .read_to_end(&mut buf)
            .await
            .map_err(|err| WebsocketError::WebsocketIOError {
                source: Box::new(err),
                #[cfg(backtrace)]
                backtrace: Backtrace::capture(),
                context: None,
            })?;

        debug!(
            "Websocket Incoming Message: {}",
            String::from_utf8_lossy(&buf)
        );

        let parsed_resp =
            serde_json::from_slice::<crate::datastructs::websocket::WebsocketMessage>(&buf)
                .map_err(|err| SerdeJSONParseError {
                    message: String::from_utf8_lossy(&buf).to_string(),
                    source: err,
                })?;

        Ok(parsed_resp)
    }
}
