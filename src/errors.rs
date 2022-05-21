#[cfg(backtrace)]
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::fmt::{
    Debug,
    Display,
    Formatter,
};

use hmac::digest::InvalidLength;
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP request error")]
    ReqwestError(reqwest::Error),

    #[error("Connection error")]
    ReqwestConnectionError,

    #[error("Connection has timed out")]
    ReqwestTimeoutError,

    #[error("Connection redirection error")]
    ReqwestRedirectError,

    #[error("Error parsing:")]
    SerdeError {
        source: serde_json::Error,
        string: String,
    },

    #[error("Could not serialize")]
    SerdeSerializationError(serde_json::Error),

    #[error("Error parsing string: ")]
    ParsingError(String),

    #[error("Error from from Coinbase Pro server: {}", .0.message)]
    CBProServerErrorVariant(#[from] CBProServerError),

    #[error("Websocket frame incorrectly sized")]
    WebsocketFrameSizeError(WebsocketFrameSizeErrorData),

    #[error("Request Builder failed to clone")]
    RequestBuilderCloningError,

    #[error("Invalid length of secret string")]
    InvalidSecretLength(#[from] InvalidLength),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        if error.is_connect() {
            return Error::ReqwestConnectionError;
        }

        if error.is_timeout() {
            return Error::ReqwestTimeoutError;
        }

        if error.is_redirect() {
            return Error::ReqwestRedirectError;
        }

        Error::ReqwestError(error)
    }
}

#[derive(Error, Debug, Clone)]
pub enum OrderError {
    #[error("Cannot apply a stop to a market order")]
    StopOnMarketOrder,

    #[error("Time in force cannot be applied to a market order")]
    TimeInForceOnMarketOrder,

    #[error("Post only is not valid on a market order")]
    PostOnlyOnMarketOrder,

    #[error("Post only cannot be applied to IOC or FOK Time and Force")]
    PostOnlyInvalid,
}

#[derive(Debug)]
pub struct WebsocketFrameSizeErrorData {
    pub len: usize,
    pub expected: usize,
    pub bytes: Vec<u8>,
    pub parsed_length_initial: Option<usize>,
    pub parsed_length: Option<usize>,
    pub parsed_masked: Option<bool>,
}

impl WebsocketFrameSizeErrorData {
    pub fn new(
        len: usize,
        expected: usize,
        bytes: Vec<u8>,
        parsed_length_initial: Option<usize>,
        parsed_length: Option<usize>,
        parsed_masked: Option<bool>,
    ) -> Self {
        WebsocketFrameSizeErrorData {
            len,
            expected,
            bytes,
            parsed_length_initial,
            parsed_length,
            parsed_masked,
        }
    }
}

impl Display for WebsocketFrameSizeErrorData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Frame initialization expected {} bytes but got {}.\n\
            Bytes: {:?}\n\
            Initial Length: {:?}\n\
            Length: {:?}\n\
            Masked: {:?}\n\
            ",
            self.expected,
            self.len,
            self.bytes,
            self.parsed_length_initial,
            self.parsed_length,
            self.parsed_masked
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Error, Clone)]
#[error("Error from coinbase server: {message}")]
pub struct CBProServerError {
    pub message: String,
}

#[derive(Debug)]
pub enum WebsocketError {
    ToDo,
    IndexingError {
        index: u8,
        #[cfg(backtrace)]
        backtrace: Backtrace,
    },
    FrameSize(UnexpectedEnd),
    MaskSize(IncorrectMaskSize),
    NoWebsocketConnectionError,
    ParseError(SerdeJSONParseError),
    URLParseError {
        source: Box<dyn std::error::Error + 'static>,
        url: String,
    },
    WebsocketIOError {
        source: Box<dyn std::error::Error + 'static>,
        #[cfg(backtrace)]
        backtrace: Backtrace,
        context: Option<HashMap<String, String>>,
    },
    WebsocketInitializationError(String),
    SocketAddressError {
        source: Box<dyn std::error::Error + 'static>,
        url: String,
    },
    NoSocketAddressError {
        url: String,
    },
    TCPConnectionError {
        source: Box<dyn std::error::Error + 'static>,
        url: String,
    },
    TLSConnectionError {
        source: Box<dyn std::error::Error + 'static>,
        #[cfg(backtrace)]
        backtrace: Backtrace,
        url: String,
    },
    WebsocketConnectionError {
        source: Box<dyn std::error::Error + 'static>,
        url: String,
    },
    WebsocketUpgradeError,
    NoDomainError {
        url: String,
    },
    StdError(Box<dyn std::error::Error + 'static>),
}

impl From<std::io::Error> for WebsocketError {
    fn from(error: std::io::Error) -> Self {
        Self::WebsocketIOError {
            source: Box::new(error),
            #[cfg(backtrace)]
            backtrace: Backtrace::capture(),
            context: None,
        }
    }
}

impl From<Box<dyn std::error::Error + 'static>> for WebsocketError {
    fn from(error: Box<dyn std::error::Error + 'static>) -> Self {
        Self::StdError(error)
    }
}

impl Display for WebsocketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            WebsocketError::IndexingError { index, .. } => {
                format!("Attempted to access out of bounds index: {}", index)
            }
            WebsocketError::FrameSize(err) => {
                format!("Could not parse Websocket Frame, unexpected size\n{0}", err)
            }
            WebsocketError::MaskSize(err) => {
                format!("Incorrect mask size:\n{}", err.received_size)
            }
            WebsocketError::NoWebsocketConnectionError => {
                format!("Must subscribe to a websocket before reading websocket frame")
            }
            WebsocketError::ParseError(err) => {
                format!(
                    "Could not parse Websocket Frame as CoinBase websocket message\n{}",
                    err.message
                )
            }
            WebsocketError::URLParseError { url, source } => {
                format!("Failed to parse URL: {}\nSource error: {}", url, source)
            }
            #[cfg(backtrace)]
            WebsocketError::WebsocketIOError {
                source,
                backtrace,
                context,
                ..
            } => {
                let mut text = format!("Websocket IO Error\nSource error: {}\n", source);
                if let Some(ctx) = context {
                    text.push_str(format!("{:?}", ctx).as_str());
                }
                #[cfg(backtrace)]
                text.push_str(format!("\nBacktrace: {}", backtrace).as_str());
                text
            }
            #[cfg(not(backtrace))]
            WebsocketError::WebsocketIOError {
                source, context, ..
            } => {
                let mut text = format!("Websocket IO Error\nSource error: {}\n", source);
                if let Some(ctx) = context {
                    text.push_str(format!("{:?}", ctx).as_str());
                }
                text
            }
            WebsocketError::WebsocketInitializationError(err) => {
                format!("Websocket initialization error: {0}", err)
            }
            WebsocketError::SocketAddressError { url, source } => {
                format!("Socket address failure. Failed to receive socket address for: {}\nSource error: {}", url, source)
            }
            WebsocketError::NoSocketAddressError { url } => {
                format!("Socket address failure. Socket address succeeded but did not return a usable address: {}", url)
            }
            WebsocketError::TCPConnectionError { url, source } => {
                format!(
                    "Error establishing TCP connection: {}\nSource error: {}",
                    url, source
                )
            }
            #[cfg(backtrace)]
            WebsocketError::TLSConnectionError {
                url,
                source,
                backtrace,
            } => {
                let mut text = format!(
                    "Error establishing TLS connection: {}\nSource error: {}\n",
                    url, source
                );
                #[cfg(backtrace)]
                text.push_str(format!("\nBacktrace: {}", backtrace).as_str());
                text
            }
            #[cfg(not(backtrace))]
            WebsocketError::TLSConnectionError { url, source } => {
                let mut text = format!(
                    "Error establishing TLS connection: {}\nSource error: {}\n",
                    url, source
                );
                text
            }
            WebsocketError::WebsocketConnectionError { url, source } => {
                format!(
                    "Error establishing Websocket connection: {}\nSource error: {}",
                    url, source
                )
            }
            WebsocketError::WebsocketUpgradeError => {
                format!("Websocket failed to upgrade")
            }
            WebsocketError::NoDomainError { url } => {
                format!("No domain found for url: {}", url)
            }
            _ => "Unimplemented Websocket Error".to_string(),
        };

        write!(f, "{}", text)
    }
}

impl From<UnexpectedEnd> for WebsocketError {
    fn from(err: UnexpectedEnd) -> Self {
        Self::FrameSize(err)
    }
}

impl From<IncorrectMaskSize> for WebsocketError {
    fn from(err: IncorrectMaskSize) -> Self {
        Self::MaskSize(err)
    }
}

impl From<SerdeJSONParseError> for WebsocketError {
    fn from(err: SerdeJSONParseError) -> Self {
        Self::ParseError(err)
    }
}

impl From<String> for WebsocketError {
    fn from(err: String) -> Self {
        Self::WebsocketInitializationError(err)
    }
}

impl std::error::Error for WebsocketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WebsocketError::URLParseError { source, .. } => Some(&**source),
            WebsocketError::SocketAddressError { source, .. } => Some(&**source),
            WebsocketError::TCPConnectionError { source, .. } => Some(&**source),
            WebsocketError::TLSConnectionError { source, .. } => Some(&**source),
            WebsocketError::WebsocketConnectionError { source, .. } => Some(&**source),
            _ => None,
        }
    }

    #[cfg(backtrace)]
    fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            WebsocketError::TLSConnectionError { backtrace, .. } => Some(backtrace),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct UnexpectedEnd {
    pub expected_length: usize,
    pub received_size: usize,
    #[cfg(backtrace)]
    pub backtrace: Backtrace,
}

impl Display for UnexpectedEnd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut text = format!(
            "\
        Expected size: {}\n\
        Received size: {}",
            self.expected_length, self.received_size,
        );
        #[cfg(backtrace)]
        text.push_str(format!("\nBacktrace: {}", self.backtrace).as_str());
        write!(f, "{}", text)
    }
}

#[derive(Debug, Error)]
#[error(
    "\
Expected size: {expected_length}\n\
Received size: {received_size}\
"
)]
pub struct IncorrectMaskSize {
    pub expected_length: usize,
    pub received_size: usize,
}

#[derive(Debug, Error)]
#[error(
    "\
Could not parse into Websocket Message: {message}
"
)]
pub struct SerdeJSONParseError {
    pub message: String,
    #[source]
    pub source: serde_json::Error,
}
