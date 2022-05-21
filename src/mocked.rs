#![cfg(feature = "mock")]

use std::any::type_name;
use std::borrow::{
    Borrow,
    BorrowMut,
};
use std::cell::RefCell;
use std::cmp::min;
use std::collections::vec_deque::VecDeque;
use std::fmt::{
    write,
    Debug,
    Display,
    Formatter,
};
use std::future::Future;
use std::io::{
    Read,
    Write,
};
use std::iter::Map;
use std::mem::swap;
use std::net::SocketAddr;
use std::ops::{
    Deref,
    DerefMut,
};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{
    Arc,
    Mutex as StdMutex,
};
use std::task::Poll::{
    Pending,
    Ready,
};
use std::task::{
    Context,
    Poll,
};
use std::{
    io,
    mem,
};

use async_trait::async_trait;
use log::debug;
use mockall::mock;
use reqwest::header::{
    AsHeaderName,
    HeaderMap,
    HeaderName,
    HeaderValue,
};
use reqwest::{
    Body,
    Error,
    IntoUrl,
    Method,
    Response,
};
use tokio::io::{
    AsyncRead,
    AsyncWrite,
    ReadBuf,
};
use tokio::net::TcpStream;
use tokio::sync::{
    Mutex,
    MutexGuard,
    OwnedMutexGuard,
    TryLockError,
};

use crate::api::{
    APIKeyData,
    AsyncIOBuilder,
};
use crate::errors::WebsocketError;
use crate::requests::SignRequest;
use crate::websocket_lite::AsyncIO;

pub struct MockClient {
    payload: Arc<RefCell<Option<MockRequestBuilder>>>,
    pub requested_url: Arc<RefCell<String>>,
}

impl MockClient {
    pub fn new() -> Self {
        Self {
            payload: Arc::new(RefCell::new(None)),
            requested_url: Arc::new(RefCell::new("".to_string())),
        }
    }

    pub fn new_mock(payload: MockRequestBuilder) -> Self {
        Self {
            payload: Arc::new(RefCell::new(Some(payload))),
            requested_url: Arc::new(RefCell::from("".to_string())),
        }
    }

    pub fn request(&self, method: Method, url: impl IntoUrl) -> MockRequestBuilder {
        self.requested_url
            .deref()
            .replace(format!("{:?}", url.into_url().map(|x| x.to_string())));
        let payload = RefCell::new(None);
        self.payload.swap(&payload);
        payload.into_inner().unwrap()
    }
}

impl Clone for MockClient {
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
            requested_url: self.requested_url.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArgumentInfo {
    pub name: String,
    pub argument_type: String,
    pub argument_value: String,
}

impl Display for ArgumentInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} = {}",
            self.name, self.argument_type, self.argument_value
        )
    }
}

impl ArgumentInfo {
    pub fn new<T: Debug>(name: &str, argument: T) -> Self {
        ArgumentInfo {
            name: name.to_string(),
            argument_type: type_name::<T>().to_string(),
            argument_value: format!("{:?}", argument),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CallInfo {
    pub method_name: String,
    pub arguments: Vec<ArgumentInfo>,
}

impl Display for CallInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Call: {}\n", self.method_name)?;
        self.arguments.iter().try_for_each(|x| write!(f, "{}\n", x))
    }
}

impl CallInfo {
    pub fn new(name: &str, arguments: Vec<ArgumentInfo>) -> Self {
        CallInfo {
            method_name: name.to_string(),
            arguments,
        }
    }

    pub fn called_with(&self, arg: (&str, &str)) -> bool {
        self.arguments
            .iter()
            .filter(|x| x.name == arg.0 && x.argument_value == arg.1)
            .collect::<Vec<&ArgumentInfo>>()
            .len()
            >= 1
    }
}

#[derive(Clone, Debug)]
pub struct MockRequestBuilder {
    pub call_info: Arc<RefCell<Vec<CallInfo>>>,
    pub payloads: Arc<RefCell<Vec<MockResponse>>>,
}

impl MockRequestBuilder {
    pub fn new_mock(payloads: Vec<MockResponse>) -> Self {
        Self {
            call_info: Arc::new(RefCell::new(vec![])),
            payloads: Arc::new(RefCell::new(payloads)),
        }
    }

    pub fn header<
        K: TryInto<HeaderName> + std::fmt::Debug + 'static,
        V: TryInto<HeaderValue> + std::fmt::Debug + 'static,
    >(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        let arg_1 = ArgumentInfo::new("key", key);
        let arg_2 = ArgumentInfo::new("value", value);
        let call_info = CallInfo::new("header", vec![arg_1, arg_2]);
        self.call_info.deref().borrow_mut().push(call_info);
        self
    }

    pub fn body(mut self, body: Body) -> Self {
        let arg_1 = ArgumentInfo::new("body", body);
        let call_info = CallInfo::new("body", vec![arg_1]);
        self.call_info.deref().borrow_mut().push(call_info);
        self
    }

    pub fn query<T: std::fmt::Debug + 'static>(mut self, query_params: &T) -> Self {
        let arg_1 = ArgumentInfo::new("query_params", query_params);
        let call_info = CallInfo::new("query", vec![arg_1]);
        self.call_info.deref().borrow_mut().push(call_info);
        self
    }

    pub fn try_clone(&self) -> Option<Self> {
        Some(self.clone())
    }

    pub fn build(&self) -> Result<MockResponse, Error> {
        let mut clone = self.clone();
        let mut test = clone.payloads.deref().borrow_mut();
        Ok(test.pop().unwrap())
    }

    pub async fn send(&self) -> Result<MockResponse, Error> {
        let mut test = self.payloads.deref().borrow_mut();
        Ok(test.pop().unwrap())
    }
}

mock! {
    pub Request {}
}

mock! {
    #[derive(Debug)]
    pub Response {
        pub async fn text(self) -> Result<String, Error>;
        pub fn headers(&self) -> &MockHeaderMap;
    }
}

mock! {
    pub HeaderMap {
        pub fn get<K: AsHeaderName + 'static>(&self, key: K) -> Option<&'static HeaderValue>;
    }
}

#[derive(Clone)]
pub struct MockTcpStream {
    pub connect_calls: Arc<Mutex<Vec<SocketAddr>>>,
}

impl MockTcpStream {
    pub fn new() -> MockTcpStream {
        Self {
            connect_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn connect(addr: SocketAddr) -> Result<MockTcpStream, WebsocketError> {
        let new_stream = Self {
            connect_calls: Arc::new(Mutex::new(vec![addr])),
        };
        Ok(new_stream)
    }
}

impl AsyncRead for MockTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        todo!()
    }
}

impl AsyncWrite for MockTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        todo!()
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        todo!()
    }
}

impl Read for MockTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

impl Write for MockTcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

#[derive(Clone)]
pub struct MockTlsConnector {
    pub connect_calls: Arc<Mutex<Vec<String>>>,
    pub mock_stream: MockTlsStream<MockTcpStream>,
}

impl MockTlsConnector {
    pub fn new(stream: &MockTlsStream<MockTcpStream>) -> MockTlsConnector {
        let new_connector = Self {
            connect_calls: Arc::new(Mutex::new(Vec::new())),
            mock_stream: stream.clone(),
        };
        new_connector
    }

    pub async fn connect<S: Read + Write>(
        self,
        url: &str,
        stream: S,
    ) -> Result<MockTlsStream<MockTcpStream>, Error> {
        let new_stream = self.mock_stream.clone();
        self.connect_calls.lock().await.push(url.to_string());
        Ok(new_stream)
    }
}

#[derive(Clone)]
pub struct MockTlsStream<T> {
    pub connect_calls: Arc<StdMutex<Vec<SocketAddr>>>,
    pub poll_write_expects: Arc<StdMutex<VecDeque<Box<dyn Fn(&[u8]) -> bool + Send>>>>,
    pub poll_read_expects: Arc<StdMutex<VecDeque<u8>>>,
    pub stream: T,
}

impl MockTlsStream<MockTcpStream> {
    pub fn new(stream: &MockTcpStream) -> Self {
        Self {
            connect_calls: Arc::new(StdMutex::new(vec![])),
            poll_write_expects: Arc::new(StdMutex::new(Default::default())),
            poll_read_expects: Arc::new(StdMutex::new(Default::default())),
            stream: stream.clone(),
        }
    }

    pub fn expect_poll_write(&mut self, closure: impl Fn(&[u8]) -> bool + Send + 'static) {
        self.poll_write_expects
            .lock()
            .unwrap()
            .push_back(Box::new(closure));
    }

    pub fn expect_poll_read(&mut self, buf: Vec<u8>) {
        let mut lock = self.poll_read_expects.lock().unwrap();
        buf.into_iter().for_each(|value| lock.push_back(value));
    }
}

impl AsyncRead for MockTlsStream<MockTcpStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        debug!("poll_read called");
        let mut lock = self.poll_read_expects.lock().unwrap();

        if lock.is_empty() {
            return Ready(Ok(()));
        }

        let return_len = min(buf.remaining(), lock.len());

        let mut ret_buf = Vec::new();
        ret_buf.reserve(return_len);

        for _ in 0..return_len {
            let byte = match lock.pop_front() {
                None => break,
                Some(byte) => byte,
            };

            debug!("Reading byte: {}", byte);
            ret_buf.push(byte);
        }

        buf.put_slice(ret_buf.as_slice());

        return Ready(Ok(()));
    }
}

impl AsyncWrite for MockTlsStream<MockTcpStream> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        debug!("pollwrite: {:?}", buf);
        let output = self.poll_write_expects.lock().unwrap().pop_front();

        let output = if let Some(output) = output {
            output
        } else {
            panic!("poll_write called more than expected");
        };

        if output(buf) {
            Ready(Ok(buf.len()))
        } else {
            panic!("Mismatch write: {}", String::from_utf8_lossy(buf))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Ready(Ok(()))
    }
}

impl Read for MockTlsStream<MockTcpStream> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for MockTlsStream<MockTcpStream> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[derive(Clone)]
pub struct MockStream {
    pub(crate) stream_contents: Arc<StdMutex<VecDeque<u8>>>,
    pub(crate) writes: Arc<StdMutex<VecDeque<u8>>>,
}

impl MockStream {
    pub fn new(buf: &[u8]) -> Self {
        Self {
            stream_contents: Arc::new(StdMutex::new(VecDeque::from_iter(buf.to_vec().into_iter()))),
            writes: Arc::new(StdMutex::new(Default::default())),
        }
    }
}

impl AsyncRead for MockStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut internal_buf = Vec::new();
        loop {
            if buf.remaining() <= internal_buf.len() {
                break;
            }

            let next = match self.stream_contents.lock().unwrap().pop_front() {
                Some(next) => next,
                None => {
                    break;
                }
            };

            internal_buf.push(next);
        }

        buf.put_slice(internal_buf.as_slice());

        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        buf.into_iter()
            .for_each(|byte| self.writes.lock().unwrap().push_back(byte.clone()));

        Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Ready(Ok(()))
    }
}

#[derive(Clone)]
pub(crate) struct MockIOBuilder {
    stream: MockStream,
}

impl MockIOBuilder {
    pub fn new(stream: &MockStream) -> Self {
        Self {
            stream: stream.clone(),
        }
    }
}

#[async_trait]
impl AsyncIOBuilder for MockIOBuilder {
    async fn new_stream(&self, url: &str) -> Result<Box<dyn AsyncIO>, WebsocketError> {
        Ok(Box::new(self.stream.clone()))
    }
}
