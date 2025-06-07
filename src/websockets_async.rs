use crate::errors::BinanceError;
use crate::config::Config;
use crate::model::{
    AccountUpdateEvent, AggrTradesEvent, BalanceUpdateEvent, BookTickerEvent, DayTickerEvent, DepthOrderBookEvent, KlineEvent, OrderBook, OrderTradeEvent, OutboundAccountPositionEvent, TradeEvent, WindowTickerEvent
};
use crate::bail;
use tokio::sync::mpsc;
use tokio::time::{self, interval};
use url::Url;
use serde::{Deserialize, Serialize};

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::net::TcpStream;

use tokio_tungstenite::{
    connect_async,
    tungstenite::{handshake::client::Response, protocol::Message},
    MaybeTlsStream, WebSocketStream,
};

use futures_util::StreamExt;
use futures_util::SinkExt;


#[allow(clippy::all)]
enum WebsocketAPI {
    Default,
    MultiStream,
    Custom(String),
}

impl WebsocketAPI {
    fn params(self, subscription: &str) -> String {
        match self {
            WebsocketAPI::Default => format!("wss://stream.binance.com/ws/{}", subscription),
            WebsocketAPI::MultiStream => format!(
                "wss://stream.binance.com/stream?streams={}",
                subscription
            ),
            WebsocketAPI::Custom(url) => format!("{}/{}", url, subscription),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebsocketEvent {
    AccountUpdate(AccountUpdateEvent),
    BalanceUpdate(BalanceUpdateEvent),
    OrderTrade(OrderTradeEvent),
    AggrTrades(AggrTradesEvent),
    Trade(TradeEvent),
    OrderBook(OrderBook),
    DayTicker(DayTickerEvent),
    DayTickerAll(Vec<DayTickerEvent>),
    WindowTicker(WindowTickerEvent),
    WindowTickerAll(Vec<WindowTickerEvent>),
    OutboundAccountPosition(OutboundAccountPositionEvent),
    Kline(KlineEvent),
    DepthOrderBook(DepthOrderBookEvent),
    BookTicker(BookTickerEvent),
}

pub struct WebSockets {
    pub socket: Option<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response)>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Events {
    DayTickerEventAll(Vec<DayTickerEvent>),
    WindowTickerEventAll(Vec<WindowTickerEvent>),
    BalanceUpdateEvent(BalanceUpdateEvent),
    DayTickerEvent(DayTickerEvent),
    WindowTickerEvent(WindowTickerEvent),
    BookTickerEvent(BookTickerEvent),
    AccountUpdateEvent(AccountUpdateEvent),
    OutboundAccountPositionEvent(OutboundAccountPositionEvent),
    OrderTradeEvent(OrderTradeEvent),
    AggrTradesEvent(AggrTradesEvent),
    TradeEvent(TradeEvent),
    KlineEvent(KlineEvent),
    OrderBook(OrderBook),
    DepthOrderBookEvent(DepthOrderBookEvent),
}

impl WebSockets {
    pub fn new() -> WebSockets
    {
        WebSockets {
            socket: None,
           
        }
    }

    pub async fn connect(&mut self, subscription: &str) -> Result<(), BinanceError> {
        self.connect_wss(&WebsocketAPI::Default.params(subscription)).await
    }

    pub async fn connect_with_config(&mut self, subscription: &str, config: &Config) -> Result<(), BinanceError> {
        self.connect_wss(&WebsocketAPI::Custom(config.ws_endpoint.clone()).params(subscription)).await
    }

    pub async fn connect_multiple_streams(&mut self, endpoints: &[String]) -> Result<(), BinanceError> {
        self.connect_wss(&WebsocketAPI::MultiStream.params(&endpoints.join("/"))).await
    }

    async fn connect_wss(&mut self, wss: &str) -> Result<(), BinanceError> {
        log::info!("connecting to {}", wss);
        let url = Url::parse(wss)?;
        match connect_async(url).await {
            Ok(answer) => {
                self.socket = Some(answer);
                Ok(())
            }
            Err(e) => bail!(format!("Error during handshake {}", e)),
        }
    }

    pub async fn disconnect(&mut self) -> Result<(), BinanceError> {
        if let Some(ref mut socket) = self.socket {
            socket.0.close(None).await?;
            return Ok(());
        }
        bail!("Not able to close the connection");
    }

    pub fn compose_event(value: serde_json::Value) -> Option<WebsocketEvent> {
        if let Ok(events) = serde_json::from_value::<Events>(value) {
            let action = match events {
                Events::DayTickerEventAll(v) => WebsocketEvent::DayTickerAll(v),
                Events::WindowTickerEventAll(v) => WebsocketEvent::WindowTickerAll(v),
                Events::BookTickerEvent(v) => WebsocketEvent::BookTicker(v),
                Events::BalanceUpdateEvent(v) => WebsocketEvent::BalanceUpdate(v),
                Events::AccountUpdateEvent(v) => WebsocketEvent::AccountUpdate(v),
                Events::OutboundAccountPositionEvent(v) => WebsocketEvent::OutboundAccountPosition(v),
                Events::OrderTradeEvent(v) => WebsocketEvent::OrderTrade(v),
                Events::AggrTradesEvent(v) => WebsocketEvent::AggrTrades(v),
                Events::TradeEvent(v) => WebsocketEvent::Trade(v),
                Events::DayTickerEvent(v) => WebsocketEvent::DayTicker(v),
                Events::WindowTickerEvent(v) => WebsocketEvent::WindowTicker(v),
                Events::KlineEvent(v) => WebsocketEvent::Kline(v),
                Events::OrderBook(v) => WebsocketEvent::OrderBook(v),
                Events::DepthOrderBookEvent(v) => WebsocketEvent::DepthOrderBook(v),
            };
            Some(action)
        } else {
            None
        }
    }

    pub async fn event_loop(&mut self, running: &AtomicBool, evt_tx: mpsc::Sender<WebsocketEvent>) -> Result<(), BinanceError> {
        let mut ping_interval = interval(Duration::from_secs(600));
        ping_interval.tick().await; // 手动跳过第一次

        while running.load(Ordering::Relaxed) {
             // 直接 match self.socket
            let socket = match &mut self.socket {
                Some(socket) => socket,
                None => bail!("self.socket is None"),
            };
            tokio::select! {
                _ = ping_interval.tick() => {
                    match socket.0.send(Message::Ping(Vec::new())).await {
                        Ok(_) => log::info!("Sent periodic ping"),
                        Err(e) => {
                            log::error!("Ping send error: {}", e);
                            bail!(format!("Ping send error: {}", e));
                        }
                    }
                },
                result = socket.0.next() => {
                    match result {
                        Some(message) => {
                            // 原有的消息处理逻辑
                            match message {
                                Ok(Message::Text(msg)) => {
                                    // 处理文本消息
                                    let value: serde_json::Value = serde_json::from_str(&msg)?;
                                    if let Some(data) = value.get("data") {
                                        if let Some(evt) = Self::compose_event(data.clone()) {
                                            match evt_tx.send(evt).await {
                                                Ok(_) => (),
                                                Err(e) => bail!(format!("Error on sending message, {}", e)),
                                            };
                                        } else {
                                            log::error!("Unknown event: {:?}", msg);
                                        }
                                    } else if let Some(evt) = Self::compose_event(value) {
                                        // match evt {
                                        //     WebsocketEvent::UserDataStreamExpired(e) => {
                                        //         bail!(format!("User data stream expired:{:#?}", e));
                                        //     },
                                        //     FuturesWebsocketEvent::ListenKeyExpired(e) => {
                                        //         bail!(format!("Listen key expired:{:#?}", e));
                                        //     },
                                        //     _ => (),
                                        // }
                                        
                                        match evt_tx.send(evt).await {
                                            Ok(_) => (),
                                            Err(e) => bail!(format!("Error on sending message, {}", e)),
                                        };
                                    } else {
                                        log::error!("Unknown event: {:?}", msg);
                                    }
                                }
                                Ok(Message::Ping(ping_data)) => {
                                    if let Some(socket) = &mut self.socket {
                                        log::info!("binance receive ping: {:?}", ping_data);
                                        socket.0.send(Message::Pong(ping_data)).await.unwrap();
                                    }
                                }
                                Ok(Message::Pong(_)) | Ok(Message::Binary(_)) | Ok(Message::Frame(_)) => (),
                                Ok(Message::Close(e)) => bail!(format!("Disconnected {:?}", e)),
                                Err(e) => {
                                    bail!(format!("Error on receiving message, {}", e));
                                }
                            }
                        }
                        None => {
                            bail!("Futures Ws Connection Closed");
                        }
                    }
                },
                _ = time::sleep(Duration::from_secs(1)) => {
                    // 每秒检查一次 running 状态
                }
            }
        }
        bail!("running loop closed");
    }
}
