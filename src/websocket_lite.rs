#[cfg(backtrace)]
use std::backtrace::Backtrace;
use std::cmp::min;
use std::collections::vec_deque::VecDeque;
use std::fmt::{
    Display,
    Formatter,
};
use std::io::{
    Error,
    ErrorKind,
};
use std::pin::Pin;
use std::sync::{
    Arc,
    Mutex,
    MutexGuard,
};
use std::task::{
    Context,
    Poll,
};
use std::{
    io,
    mem,
};

use rand::{
    RngCore,
    SeedableRng,
};
use reqwest::Url;
use sha1::{
    Digest,
    Sha1,
};
use tokio::io::{
    AsyncRead,
    AsyncReadExt,
    AsyncWrite,
    AsyncWriteExt,
    ReadBuf,
};

use crate::errors::WebsocketError::{
    NoDomainError,
    TLSConnectionError,
};
use crate::errors::{
    IncorrectMaskSize,
    WebsocketError,
};

pub struct ParsedFrame {
    _final_flag: bool,
    _mask: Option<[u8; 4]>,
    pub(crate) payload: Vec<u8>,
}

pub enum ParserState {
    FinalAndOpcode,
    MaskAndLength,
    MediumLength(u8),
    LongLength(u8),
    Mask(u8),
    Payload(u8),
}

impl Display for ParserState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            ParserState::FinalAndOpcode => format!("FinalAndOpcode"),
            ParserState::MaskAndLength => format!("MaskAndLength"),
            ParserState::MediumLength(index) => format!("MediumLength, Index: {}", index),
            ParserState::LongLength(index) => format!("LongLength, Index: {}", index),
            ParserState::Mask(index) => format!("Mask, Index: {}", index),
            ParserState::Payload(index) => format!("Payload, Index: {}", index),
        };
        write!(f, "{}", out)
    }
}

pub enum FrameLength {
    Short([u8; 1]),
    Medium([u8; 3]),
    Long([u8; 9]),
}

impl FrameLength {
    pub fn to_vec(self) -> Vec<u8> {
        match self {
            FrameLength::Short(bytes) => bytes.to_vec(),
            FrameLength::Medium(bytes) => bytes.to_vec(),
            FrameLength::Long(bytes) => bytes.to_vec(),
        }
    }
}

impl From<u64> for FrameLength {
    fn from(len: u64) -> Self {
        if len < 126 {
            Self::Short([len as u8])
        } else if len <= u16::MAX as u64 {
            let bytes = len.to_be_bytes();
            Self::Medium([126, bytes[0], bytes[1]])
        } else {
            let bytes = len.to_be_bytes();
            Self::Long([
                127, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ])
        }
    }
}

pub struct Frame {
    data: Vec<u8>,
}

impl Frame {
    fn apply_mask(data: &mut Vec<u8>, payload: &[u8], mask: &[u8; 4]) {
        payload
            .into_iter()
            .enumerate()
            .for_each(|(index, byte)| data.push(byte ^ mask[index % 4]));
    }
}

impl From<&[u8]> for Frame {
    fn from(payload: &[u8]) -> Self {
        let mut len = FrameLength::from(payload.len() as u64).to_vec();
        len[0] += 0b10000000;
        let mut data: Vec<u8> = Vec::with_capacity(5 + len.len() + payload.len());

        // Final frame and opcode 1
        data.push(0b10000001);
        data.append(&mut len);

        let mut mask = [0u8; 4];
        rand::rngs::StdRng::from_entropy().fill_bytes(&mut mask);
        data.extend_from_slice(&mask);

        Frame::apply_mask(&mut data, payload, &mask);

        Frame { data }
    }
}

pub struct FrameParser {
    current_state: ParserState,
    final_frame: bool,
    op_code: u8,
    mask: Option<[u8; 4]>,
    length: u64,
    payload: Vec<u8>,
}

impl Default for FrameParser {
    fn default() -> Self {
        Self {
            current_state: ParserState::FinalAndOpcode,
            final_frame: false,
            op_code: 0,
            mask: None,
            length: 0,
            payload: vec![],
        }
    }
}

impl FrameParser {
    pub fn take_frame_then_reset(&mut self) -> Result<ParsedFrame, WebsocketError> {
        let mut payload = Vec::new();
        mem::swap(&mut payload, &mut self.payload);
        self.current_state = ParserState::FinalAndOpcode;
        self.final_frame = false;
        self.op_code = 0;
        self.mask = None;
        self.length = 0;

        Ok(ParsedFrame {
            _final_flag: self.final_frame,
            _mask: self.mask,
            payload,
        })
    }

    pub fn process_byte(&mut self, byte: &u8) -> Result<Option<ParsedFrame>, WebsocketError> {
        match self.current_state {
            ParserState::FinalAndOpcode => self.final_and_opcode(byte),
            ParserState::MaskAndLength => self.mask_and_length(byte),
            ParserState::MediumLength(index) => self.medium_length(byte, &index),
            ParserState::LongLength(index) => self.long_length(byte, &index),
            ParserState::Mask(index) => self.mask(byte, &index),
            ParserState::Payload(index) => self.payload(byte, &index),
        }
    }

    pub fn final_and_opcode(&mut self, byte: &u8) -> Result<Option<ParsedFrame>, WebsocketError> {
        self.final_frame = byte & 0b10000000 > 0;
        self.op_code = byte & 0b00001111;
        self.current_state = ParserState::MaskAndLength;
        Ok(None)
    }

    pub fn mask_and_length(&mut self, byte: &u8) -> Result<Option<ParsedFrame>, WebsocketError> {
        if byte & 0b10000000 > 0 {
            self.mask = Some([0; 4]);
        }

        let length = byte & 0b01111111;

        if length > 127 {
            return Err(WebsocketError::ToDo);
        }
        if length == 127 {
            self.current_state = ParserState::LongLength(0);
        }
        if length == 126 {
            self.current_state = ParserState::MediumLength(0);
        }
        if length == 0 {
            let frame = self.take_frame_then_reset()?;
            self.current_state = ParserState::FinalAndOpcode;
            return Ok(Some(frame));
        }
        if length < 126 && self.mask.is_some() {
            self.length = length as u64;
            self.current_state = ParserState::Mask(0);
        }
        if length < 126 && self.mask.is_none() {
            self.length = length as u64;
            self.current_state = ParserState::Payload(0);
        }

        Ok(None)
    }

    pub fn medium_length(
        &mut self,
        byte: &u8,
        index: &u8,
    ) -> Result<Option<ParsedFrame>, WebsocketError> {
        if *index == 0 {
            self.length = 0;
        }

        if *index != 0 {
            self.length = self.length << 8;
        }

        self.length += *byte as u64;

        if *index == 0 {
            self.current_state = ParserState::MediumLength(1);
        }

        if *index != 0 && self.mask.is_some() {
            self.current_state = ParserState::Mask(0)
        }

        if *index != 0 && self.mask.is_none() && self.length != 0 {
            self.current_state = ParserState::Payload(0);
        }

        if *index != 0 && self.mask.is_none() && self.length == 0 {
            let frame = self.take_frame_then_reset()?;
            return Ok(Some(frame));
        }

        Ok(None)
    }

    pub fn long_length(
        &mut self,
        byte: &u8,
        index: &u8,
    ) -> Result<Option<ParsedFrame>, WebsocketError> {
        if *index == 0 {
            self.length = 0;
        }

        if *index != 0 {
            self.length = self.length << 8;
        }

        self.length += *byte as u64;

        if *index < 7 {
            self.current_state = ParserState::LongLength(index + 1);
        }

        if *index >= 7 && self.mask.is_some() {
            self.current_state = ParserState::Mask(0);
        }

        if *index >= 7 && self.mask.is_none() && self.length != 0 {
            self.current_state = ParserState::Payload(0);
        }

        if *index >= 7 && self.mask.is_none() && self.length == 0 {
            let frame = self.take_frame_then_reset()?;
            return Ok(Some(frame));
        }

        Ok(None)
    }

    pub fn mask(&mut self, byte: &u8, index: &u8) -> Result<Option<ParsedFrame>, WebsocketError> {
        if self.mask.is_none() || *index >= 4 {
            return Err(WebsocketError::MaskSize(IncorrectMaskSize {
                expected_length: 0,
                received_size: 0,
            }));
        }

        let mut mask = self.mask.unwrap();
        mask[*index as usize] = byte.clone();
        self.mask = Some(mask);

        if *index < 3 {
            self.current_state = ParserState::Mask(index + 1);
        }

        if *index == 3 && self.length != 0 {
            self.current_state = ParserState::Payload(0);
        }

        if *index == 3 && self.length == 0 {
            let frame = self.take_frame_then_reset()?;
            return Ok(Some(frame));
        }

        Ok(None)
    }

    pub fn payload(
        &mut self,
        byte: &u8,
        index: &u8,
    ) -> Result<Option<ParsedFrame>, WebsocketError> {
        let default = [0, 0, 0, 0];

        let mask = self
            .mask
            .unwrap_or(default)
            .as_slice()
            .get(*index as usize)
            .ok_or(WebsocketError::IndexingError {
                index: index.clone(),
                #[cfg(backtrace)]
                backtrace: Backtrace::capture(),
            })?
            .clone();

        self.payload.push(byte ^ mask);

        if self.payload.len() == self.length as usize {
            let frame = self.take_frame_then_reset()?;
            return Ok(Some(frame));
        }

        if *index == 3 {
            self.current_state = ParserState::Payload(0);
        }

        if *index < 3 {
            self.current_state = ParserState::Payload(index + 1);
        }

        if *index > 3 {
            return Err(WebsocketError::IndexingError {
                index: index.clone(),
                #[cfg(backtrace)]
                backtrace: Backtrace::capture(),
            });
        }

        Ok(None)
    }
}

pub trait AsyncIO: AsyncRead + AsyncWrite + Unpin {}

macro_rules! ok_or_return_poll_poison {
    ($result:expr) => {{
        match $result {
            Ok(res) => res,
            Err(_) => {
                return Poll::Ready(Err(io::Error::new(
                    ErrorKind::PermissionDenied,
                    "Mutex has been poisoned!",
                )));
            }
        }
    }};
}

impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncIO for T {}

pub struct WebsocketStreamConnector {
    check_sec_accept: bool,
}

impl Default for WebsocketStreamConnector {
    fn default() -> Self {
        Self {
            check_sec_accept: true,
        }
    }
}

impl WebsocketStreamConnector {
    pub(crate) fn new_no_sec() -> Self {
        Self {
            check_sec_accept: false,
        }
    }

    pub async fn connect<T>(
        &self,
        mut stream: T,
        url: &Url,
    ) -> Result<WebsocketStream<T>, WebsocketError>
    where
        T: AsyncIO,
    {
        let domain = url.domain().ok_or(NoDomainError {
            url: url.to_string(),
        })?;
        let port = url.socket_addrs(|| None).unwrap()[0].port();

        // let port_str = match port {
        //     None => {"".to_string()}
        //     Some(port) => {format!(":{}", port)}
        // };

        let sec_key = Self::new_sec_key();
        let sec_accept = Self::sec_accept(&sec_key);

        let host_str = format!("Host: {}:{}\r\n", domain, port);
        let request_str = format!(
            "{}{}{}{}{}{}{}{}{}{}\r\n",
            "GET / HTTP/1.1\r\n",
            host_str,
            "User-Agent: rust\r\n",
            "Upgrade: websocket\r\n",
            "Connection: Upgrade\r\n",
            "Sec-WebSocket-Key: ",
            sec_key,
            "\r\n",
            "Sec-WebSocket-Protocol: chat\r\n",
            "Sec-WebSocket-Version: 13\r\n",
        );

        stream
            .write(request_str.as_bytes())
            .await
            .map_err(|err| TLSConnectionError {
                source: Box::new(err),
                #[cfg(backtrace)]
                backtrace: Backtrace::capture(),
                url: "".to_string(),
            })?;

        let response = Self::read_to_end_http(&mut stream).await.unwrap();

        if self.check_sec_accept {
            println!("{:?}", response);

            let location = response
                .find("Sec-WebSocket-Accept:")
                .ok_or(WebsocketError::WebsocketUpgradeError)?;

            let new_line = response[(location + 21)..]
                .find("\r\n")
                .ok_or(WebsocketError::WebsocketUpgradeError)?
                + location
                + 21;

            if response[(location + 21)..new_line].trim() != &sec_accept {
                return Err(WebsocketError::WebsocketUpgradeError);
            }
        }

        Ok(WebsocketStream {
            stream,
            read_parser: Arc::new(Mutex::new(FrameParser::default())),
            frame_buffer: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    async fn read_to_end_http<T>(stream: &mut T) -> Option<String>
    where
        T: AsyncIO,
    {
        let mut output = String::new();

        loop {
            let byte = stream.read_u8().await.ok()?;
            output.push(char::from(byte));

            if output.ends_with("\r\n\r\n") {
                break;
            }
        }

        Some(output)
    }

    pub(crate) fn sec_accept(sec_key: &str) -> String {
        let mut guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes().to_vec();
        let mut sec_accept = sec_key.as_bytes().to_vec();
        sec_accept.append(&mut guid);
        let mut hasher = Sha1::new();
        hasher.update(sec_accept);
        let hash = hasher.finalize();
        base64::encode(hash)
    }

    pub(crate) fn new_sec_key() -> String {
        let mut bytes = [0u8; 2];
        rand::rngs::StdRng::from_entropy().fill_bytes(&mut bytes);
        base64::encode(bytes)
    }
}

pub struct WebsocketStream<T>
where
    T: AsyncIO,
{
    stream: T,
    read_parser: Arc<Mutex<FrameParser>>,
    frame_buffer: Arc<Mutex<VecDeque<VecDeque<u8>>>>,
}

impl<T> WebsocketStream<T>
where
    T: AsyncIO,
{
    #[allow(unused)]
    fn take_buffer_lock(&mut self) -> Result<MutexGuard<VecDeque<VecDeque<u8>>>, std::io::Error> {
        match self.frame_buffer.lock() {
            Ok(lock) => Ok(lock),
            Err(_) => Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "Mutex has been poisoned!",
            )),
        }
    }
}

impl<T> AsyncRead for WebsocketStream<T>
where
    T: AsyncIO,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // Poll read until a frame is parsed.
        loop {
            {
                let frame_buffer = ok_or_return_poll_poison!(self.frame_buffer.lock());

                // If we have a frame to give, break and give it to the reader.
                if frame_buffer.len() > 0 {
                    break;
                }
            }

            let mut arr = [0u8; 1024];
            let mut internal_buffer = ReadBuf::new(arr.as_mut_slice());

            let stream_read_result = Pin::new(&mut self.stream).poll_read(cx, &mut internal_buffer);

            if stream_read_result.is_pending() {
                return stream_read_result;
            }

            let mut lock = ok_or_return_poll_poison!(self.read_parser.lock());

            for byte in internal_buffer.filled() {
                let maybe_frame = match lock.process_byte(byte) {
                    Ok(maybe_frame) => maybe_frame,
                    Err(_) => {
                        return Poll::Ready(Err(io::Error::from(ErrorKind::Other)));
                    }
                };

                if let Some(frame) = maybe_frame {
                    let mut frame_buffer = ok_or_return_poll_poison!(self.frame_buffer.lock());
                    let queue = VecDeque::from(frame.payload);
                    frame_buffer.push_back(queue);
                }
            }
        }

        let mut frame_buffer = ok_or_return_poll_poison!(self.frame_buffer.lock());

        if frame_buffer.len() == 0 {
            return Poll::Ready(Ok(()));
        }

        let mut payload = frame_buffer.pop_front().unwrap();

        let range = min(payload.len(), buf.remaining());
        let mut new_slice = Vec::new();
        new_slice.reserve(range);

        loop {
            if buf.remaining() <= new_slice.len() {
                break;
            }

            let byte = match payload.pop_front() {
                None => {
                    break;
                }
                Some(byte) => byte,
            };

            new_slice.push(byte);
        }

        let bytes_written = new_slice.len();
        buf.put_slice(new_slice.as_mut_slice());

        if bytes_written != 0 {
            frame_buffer.push_front(payload);
        }

        Poll::Ready(Ok(()))
    }
}

impl<T> AsyncWrite for WebsocketStream<T>
where
    T: AsyncIO,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let frame = Frame::from(buf).data;
        let header_size = frame.len() - buf.len();

        let mut parser = FrameParser::default();
        let mut parsed_frame: ParsedFrame = ParsedFrame {
            _final_flag: false,
            _mask: None,
            payload: vec![],
        };
        for byte in &frame {
            if let Ok(Some(frame)) = parser.process_byte(&byte) {
                parsed_frame = frame;
            }
        }

        println!("{}", String::from_utf8_lossy(&parsed_frame.payload));

        match Pin::new(&mut (self.stream)).poll_write(cx, &frame) {
            Poll::Ready(res) => match res {
                Ok(len) => {
                    if len <= header_size {
                        Poll::Ready(Ok(0))
                    } else {
                        Poll::Ready(Ok(len - header_size))
                    }
                }
                Err(err) => Poll::Ready(Err(err)),
            },
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut (self.stream)).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut (self.stream)).poll_shutdown(cx)
    }
}

//
// #[derive(Clone)]
// pub struct WebsocketConnection<T: AsyncReadExt + ReadHTTPResponse + AsyncWriteExt + Unpin> {
//     stream: T,
// }
//
// impl<T: AsyncReadExt + ReadHTTPResponse + AsyncWriteExt + Unpin> WebsocketConnection<T> {
//     pub async fn new(stream: T, url: impl IntoUrl) -> Result<Self, WebsocketError> {
//         WebsocketConnection { stream }
//             .upgrade_to_websocket(url)
//             .await
//     }
//
//     async fn upgrade_to_websocket(mut self, url: impl IntoUrl) -> Result<Self, WebsocketError> {
//         let url_string = url.as_str().to_string();
//         let url_checked = url.into_url().map_err(|err| URLParseError {
//             source: Box::new(err),
//             url: url_string.clone(),
//         })?;
//         let domain = url_checked.domain().ok_or(NoDomainError {
//             url: url_string.clone(),
//         })?;
//         let port = url_checked.socket_addrs(|| None).unwrap()[0].port();
//
//         // let port_str = match port {
//         //     None => {"".to_string()}
//         //     Some(port) => {format!(":{}", port)}
//         // };
//
//         let host_str = format!("Host: {}:{}\r\n", domain, port);
//         let request_str = format!(
//             "{}{}{}{}{}{}{}{}\r\n",
//             "GET / HTTP/1.1\r\n",
//             host_str,
//             "User-Agent: rust\r\n",
//             "Upgrade: websocket\r\n",
//             "Connection: Upgrade\r\n",
//             "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==\r\n",
//             "Sec-WebSocket-Protocol: chat\r\n",
//             "Sec-WebSocket-Version: 13\r\n",
//         );
//
//         self.stream
//             .write(request_str.as_bytes())
//             .await
//             .map_err(|err| TLSConnectionError {
//                 source: Box::new(err),
//                 #[cfg(backtrace)]
//                 backtrace: Backtrace::capture(),
//                 url: "".to_string(),
//             })?;
//
//         self.stream.read_to_end_of_response().await?;
//         Ok(self)
//     }
// }
//
// #[async_trait]
// pub trait ReadHTTPResponse {
//     async fn read_to_end_of_response(&mut self) -> Result<String, WebsocketError>;
// }
//
// #[async_trait]
// impl<T: AsyncRead + Unpin + Send> ReadHTTPResponse for T {
//     async fn read_to_end_of_response(&mut self) -> Result<String, WebsocketError> {
//         let mut buf = String::new();
//         let mut eof = false;
//
//         let mut byte_buf = [0];
//
//         while !eof {
//             let byte = self.read_u8().await.map_err(|err| WebsocketIOError {
//                 source: Box::new(err),
//                 #[cfg(backtrace)]
//                 backtrace: Backtrace::capture(),
//                 context: Some(HashMap::from([
//                     ("Buffer State".to_string(), format!("\"{}\"", buf.clone())),
//                     ("Buffer len".to_string(), format!("\"{}\"", buf.len())),
//                 ])),
//             })?;
//             buf.push(byte as char);
//             if buf.len() >= 4 && buf[buf.len() - 4..] == *"\r\n\r\n" {
//                 eof = true;
//             }
//         }
//
//         Ok(buf)
//     }
// }
//
// impl<T: AsyncReadExt + AsyncWriteExt + Unpin + Send> WebsocketConnection<T> {
//     pub async fn write_string_as_frame(self: &mut Self, payload: String) -> std::io::Result<usize> {
//         self.stream
//             .write(Frame::from_payload_masked(payload).into_vec().as_slice())
//             .await
//     }
//
//     pub async fn read_as_payload_string(self: &mut Self) -> Result<String, WebsocketError> {
//         Ok(String::from_utf8_lossy(
//             Frame::new_from_stream(self)
//                 .await?
//                 .unmasked_payload()
//                 .as_slice(),
//         )
//         .to_string())
//     }
// }
//
// // impl<T: AsyncReadExt + AsyncWriteExt + Unpin + Send> WebsocketConnection<T> {
// //     pub async fn read_frame(mut self: Self) -> Result<Frame, WebsocketError> {
// //         let mut buffer = Vec::new();
// //         self.read_buf(&mut buffer)
// //             .await
// //             .map_err(|x| WebsocketIOError)?;
// //         Frame::new(buffer)
// //     }
// // }
//
// impl<T: AsyncReadExt + AsyncWriteExt + Unpin + Send> AsyncRead for WebsocketConnection<T> {
//     fn poll_read(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &mut ReadBuf<'_>,
//     ) -> Poll<std::io::Result<()>> {
//         Pin::new(&mut self.stream).poll_read(cx, buf)
//     }
// }
//
// impl<T: AsyncReadExt + AsyncWriteExt + Unpin + Send> AsyncWrite for WebsocketConnection<T> {
//     fn poll_write(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &[u8],
//     ) -> Poll<Result<usize, std::io::Error>> {
//         Pin::new(&mut self.stream).poll_write(cx, buf)
//     }
//
//     fn poll_flush(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         Pin::new(&mut self.stream).poll_flush(cx)
//     }
//
//     fn poll_shutdown(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Result<(), std::io::Error>> {
//         Pin::new(&mut self.stream).poll_shutdown(cx)
//     }
// }
