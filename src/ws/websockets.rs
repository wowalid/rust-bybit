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
use tokio::time::{timeout as tokio_timeout, sleep};
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
    handler: Box<dyn FnMut(WE) -> HandlerFuture + 'a + Send>,
    pub last_message_time: Option<std::time::SystemTime>,
    pub timeout: Option<std::time::Duration>, 
}

impl<'a, WE: serde::de::DeserializeOwned> WebSockets<'a, WE> {
    /// New websocket holder with default configuration
    /// # Examples
    /// see examples/binance_websockets.rs
    pub fn new<Callback>(handler: Callback, timeout: Option<std::time::Duration>) -> WebSockets<'a, WE>
    where
        Callback: FnMut(WE) -> HandlerFuture + 'a + Send,
    {
        Self::new_with_options(handler, timeout)
    }

    /// New websocket holder with provided configuration
    /// # Examples
    /// see examples/binance_websockets.rs
    pub fn new_with_options<Callback>(handler: Callback, timeout: Option<std::time::Duration>) -> WebSockets<'a, WE>
    where
        Callback: FnMut(WE) -> HandlerFuture + 'a + Send,
    {
        WebSockets {
            socket: None,
            handler: Box::new(handler),
            last_message_time: None,
            timeout,
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

                if let Some(timeout_duration) = self.timeout {
                    // Check if we need to timeout due to inactivity
                    if let Some(last_message_time) = self.last_message_time {
                        let now = SystemTime::now();
                        if now.duration_since(last_message_time)?.as_secs() > timeout_duration.as_secs() {
                            println!("Timeout reached, closing connection.");
                            self.disconnect().await?;
                            break;
                        }
                    }
                }


                let message = tokio_timeout(std::time::Duration::from_secs(5), socket.next()).await;

                match message {
                    Ok(Some(Ok(msg))) => {
                        self.last_message_time = Some(SystemTime::now()); // Update last message time

                        match msg {
                            Message::Text(text) => {
                                if !text.is_empty() {


                                    if text.contains("subscribe") {
                                        continue;
                                    }
            
                                    if text.contains("auth") {
                                        continue;
                                    }
                                    let event: WE = from_str(&text)?;
                                    (self.handler)(event).await?;
                                }
                            }
                            Message::Ping(_) => {
                                socket.send(Message::Pong(Vec::new())).await?;
                            }
                            Message::Pong(_) => {
                                // Handle pong if needed
                            }
                            Message::Close(_) => {
                                println!("Socket closed");
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    Ok(Some(Err(e))) => {
                        println!("WebSocket error: {:?}", e);
                        return Err(Error::Msg("WebSocket error".to_string()));
                    }
                    Err(_) => {
                        // Timeout reached waiting for the message, check loop condition.
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
