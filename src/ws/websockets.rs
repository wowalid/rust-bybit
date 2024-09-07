use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use futures::future::BoxFuture;
use futures::{SinkExt, StreamExt};
use serde_json::{from_str, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream};
use url::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

use crate::ws::Op;

use super::{auth_req, Credentials};

#[derive(Debug, Deserialize, Error)]
#[error("label: {label}, msg: {message}")]
pub struct GateIOContentError {
    pub label: String,
    pub message: String,
}

const MAINNET_SPOT: &str = "wss://stream.bybit.com/v5/public/spot";
const MAINNET_PRIVATE: &str = "wss://stream.bybit.com/v5/private";

/// First errors are technical errors
/// All unhandled gate-io content errors are GateIOError
/// The rest are gate-io content errors that are properly handled
/// Unhandled gate-io errors are Msg
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ReqError(#[from] reqwest::Error),
    #[error(transparent)]
    InvalidHeaderError(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    UrlParserError(#[from] url::ParseError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Qs(#[from] serde_qs::Error),
    #[error(transparent)]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
    #[error(transparent)]
    TimestampError(#[from] std::time::SystemTimeError),
    #[error(transparent)]
    UTF8Err(#[from] std::str::Utf8Error),
    #[error("{response}")]
    GateIOError {
        #[from]
        response: GateIOContentError,
    },
    #[error("invalid listen key : {0}")]
    InvalidListenKey(String),
    #[error("unknown symbol {0}")]
    UnknownSymbol(String),
    #[error("{msg}")]
    InvalidOrderError { msg: String },
    #[error("invalid price")]
    InvalidPrice,
    #[error("invalid period {0}")]
    InvalidPeriod(String),
    #[error("internal server error")]
    InternalServerError,
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Msg(String),
}

/// Custom error messages
pub mod error_messages {
    pub const INVALID_PRICE: &str = "Invalid price.";
}

pub type Result<T> = core::result::Result<T, Error>;



pub static ORDERBOOK_ENDPOINT: &str = "spot.order_book";
type HandlerFuture = BoxFuture<'static, Result<()>>;

pub struct WebSockets<'a, WE> {
    pub socket: Option<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response)>,
    handler: Box<dyn FnMut(WE) -> HandlerFuture + 'a + Send>
}

impl<'a, WE: serde::de::DeserializeOwned> WebSockets<'a, WE> {
    /// New websocket holder with default configuration
    /// # Examples
    /// see examples/binance_websockets.rs
    pub fn new<Callback>(handler: Callback) -> WebSockets<'a, WE>
    where
        Callback: FnMut(WE) -> HandlerFuture + 'a + Send,
    {
        Self::new_with_options(handler)
    }

    /// New websocket holder with provided configuration
    /// # Examples
    /// see examples/binance_websockets.rs
    pub fn new_with_options<Callback>(handler: Callback) -> WebSockets<'a, WE>
    where
        Callback: FnMut(WE) -> HandlerFuture + 'a + Send,
    {
        WebSockets {
            socket: None,
            handler: Box::new(handler)
        }
    }

  

    /// Connect to a websocket endpoint
    pub async fn connect(&mut self) -> Result<()> {
        let wss: String = format!("{}", MAINNET_SPOT.to_string());
        println!("{:?}", wss);
        let url = Url::parse(&wss)?;

        self.handle_connect(url).await
    }


    pub async fn connect_private(&mut self) -> Result<()> {
        let wss: String = format!("{}", MAINNET_PRIVATE.to_string());
        println!("{:?}", wss);
        let url = Url::parse(&wss)?;

        self.handle_connect(url).await
    }

    async fn handle_connect(&mut self, url: Url) -> Result<()> {
        match connect_async(url).await {
            Ok(answer) => {
                self.socket = Some(answer);
                Ok(())
            }
            Err(e) => Err(Error::Msg(format!("Error during handshake {e}"))),
        }
    }



    pub async fn subscribe_orders(
        &mut self,
        credentials : &Credentials
    ) -> Result<()> {
        if let Some((ref mut socket, _)) = self.socket {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let t = since_the_epoch.as_secs();

            let req = auth_req(credentials);

            socket
            .send(Message::Text(req))
            .await?;

            let topic = "order".to_string();

            let sub = Op {
                op: "subscribe",
                args: vec![topic],
            };


            socket
                .send(Message::Text(serde_json::to_string(&sub)?))
                .await?;
            Ok(())
        } else {
            Err(Error::Msg("Not able to send the message".to_string()))
        }
    }

    pub async fn subscribe_orderbook(
        &mut self,
        pair: String,
        level: String
    ) -> Result<()> {
        if let Some((ref mut socket, _)) = self.socket {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let t = since_the_epoch.as_secs();

            let topic = format!("orderbook.{level}.{pair}");

            let sub = Op {
                op: "subscribe",
                args: vec![topic],
            };
           
            socket
                .send(Message::Text(serde_json::to_string(&sub)?))
                .await?;
            Ok(())
        } else {
            Err(Error::Msg("Not able to send the message".to_string()))
        }
    }

    /// Disconnect from the endpoint
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut socket) = self.socket {
            socket.0.close(None).await?;
            Ok(())
        } else {
            Err(Error::Msg("Not able to close the connection".to_string()))
        }
    }

    pub fn socket(&self) -> &Option<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response)> {
        &self.socket
    }

    pub async fn event_loop(&mut self, running: &AtomicBool) -> Result<()> {
        while running.load(Ordering::Relaxed) {
            if let Some((ref mut socket, _)) = self.socket {
                // TODO: return error instead of panic?
                let message = socket.next().await.unwrap()?;

                match message {
                    Message::Text(msg) => {
                        if msg.is_empty() {
                            return Ok(());
                        }

                        if msg.contains("subscribe") {
                            continue;
                        }

                        if msg.contains("auth") {
                            continue;
                        }

                        let event: WE = from_str(msg.as_str())?;
                        (self.handler)(event).await?;
                    }
                    Message::Ping(payload) => {
                        if let Some((ref mut socket, _)) = self.socket {
                            socket.send(Message::Pong(Vec::new())).await?;
                        }
                    }
                    Message::Pong(_) => {
                        // Pong message received, handle if necessary
                    }

                    Message::Close(e) => {
                        return Err(Error::Msg(format!("Disconnected {e:?}")));
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
