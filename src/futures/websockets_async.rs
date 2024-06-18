use crate::errors::BinanceError;
use crate::config::Config;
use crate::model::{
    AccountUpdateEvent, AggrTradesEvent, ContinuousKlineEvent, DayTickerEvent, DepthOrderBookEvent, FutureBalanceUpdateEvent, FuturesBookTickerEvent, IndexKlineEvent, IndexPriceEvent, KlineEvent, LiquidationEvent, MarkPriceEvent, MiniTickerEvent, OrderBook, TradeEvent, UserDataStreamExpiredEvent
};
use crate::futures::model;
use crate::bail;
use tokio::sync::mpsc;
use url::Url;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpStream;

use tokio_tungstenite::{
    connect_async,
    tungstenite::{handshake::client::Response, protocol::Message},
    MaybeTlsStream, WebSocketStream,
};
use futures_util::StreamExt;
use futures_util::SinkExt;
#[allow(clippy::all)]
pub enum FuturesWebsocketAPI {
    Default,
    MultiStream,
    Custom(String),
}

pub enum FuturesMarket {
    USDM,
    COINM,
    Vanilla,
}

impl FuturesWebsocketAPI {
    pub fn params(self, market: &FuturesMarket, subscription: &str) -> String {
        let baseurl = match market {
            FuturesMarket::USDM => "wss://fstream.binance.com",
            FuturesMarket::COINM => "wss://dstream.binance.com",
            FuturesMarket::Vanilla => "wss://vstream.binance.com",
        };

        match self {
            FuturesWebsocketAPI::Default => {
                format!("{}/ws/{}", baseurl, subscription)
            }
            FuturesWebsocketAPI::MultiStream => {
                format!("{}/stream?streams={}", baseurl, subscription)
            }
            FuturesWebsocketAPI::Custom(url) => url,
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FuturesWebsocketEvent {
    AccountUpdate(AccountUpdateEvent),
    OrderTrade(model::OrderTradeEvent),
    AggrTrades(AggrTradesEvent),
    Trade(TradeEvent),
    OrderBook(OrderBook),
    DayTicker(DayTickerEvent),
    MiniTicker(MiniTickerEvent),
    MiniTickerAll(Vec<MiniTickerEvent>),
    IndexPrice(IndexPriceEvent),
    MarkPrice(MarkPriceEvent),
    MarkPriceAll(Vec<MarkPriceEvent>),
    DayTickerAll(Vec<DayTickerEvent>),
    Kline(KlineEvent),
    ContinuousKline(ContinuousKlineEvent),
    BalanceUpdateEvent(FutureBalanceUpdateEvent),
    IndexKline(IndexKlineEvent),
    Liquidation(LiquidationEvent),
    DepthOrderBook(DepthOrderBookEvent),
    BookTicker(FuturesBookTickerEvent),
    UserDataStreamExpiredEvent(UserDataStreamExpiredEvent),
}

pub struct FuturesAsyncWebSockets {
    pub socket: Option<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response)>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum FuturesEvents {
    BookTickerEvent(FuturesBookTickerEvent),
    OrderTradeEvent(model::OrderTradeEvent),
    
    Vec(Vec<DayTickerEvent>),
    DayTickerEvent(DayTickerEvent),
    MiniTickerEvent(MiniTickerEvent),
    VecMiniTickerEvent(Vec<MiniTickerEvent>),
    AccountUpdateEvent(AccountUpdateEvent),
    BalanceUpdateEvent(FutureBalanceUpdateEvent),
    AggrTradesEvent(AggrTradesEvent),
    IndexPriceEvent(IndexPriceEvent),
    MarkPriceEvent(MarkPriceEvent),
    VecMarkPriceEvent(Vec<MarkPriceEvent>),
    TradeEvent(TradeEvent),
    KlineEvent(KlineEvent),
    ContinuousKlineEvent(ContinuousKlineEvent),
    IndexKlineEvent(IndexKlineEvent),
    LiquidationEvent(LiquidationEvent),
    OrderBook(OrderBook),
    DepthOrderBookEvent(DepthOrderBookEvent),
    UserDataStreamExpiredEvent(UserDataStreamExpiredEvent),
}

impl FuturesAsyncWebSockets {
    pub fn new() -> FuturesAsyncWebSockets
    {
        FuturesAsyncWebSockets {
            socket: None
        }
    }

    pub async fn connect(&mut self, market: &FuturesMarket, subscription: &str) -> Result<(), BinanceError> {
        self.connect_wss(&FuturesWebsocketAPI::Default.params(market, subscription)).await
    }

    pub async fn connect_with_config(
        &mut self, market: &FuturesMarket, subscription: &str, config: & Config,
    ) -> Result<(), BinanceError> {
        self.connect_wss(
            &FuturesWebsocketAPI::Custom(config.ws_endpoint.clone()).params(market, subscription),
        ).await
    }

    pub async fn connect_multiple_streams(
        &mut self, market: &FuturesMarket, endpoints: &[String],
    ) -> Result<(), BinanceError> {
        self.connect_wss(&FuturesWebsocketAPI::MultiStream.params(market, &endpoints.join("/"))).await
    }

    async fn connect_wss(&mut self, wss: &str) -> Result<(), BinanceError> {
        let url = Url::parse(wss)?;
        match connect_async(url).await {
            Ok(tuple) => {
                self.socket = Some(tuple);
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

    pub fn compose_event(value: serde_json::Value) -> Option<FuturesWebsocketEvent> {
        if let Ok(events) = serde_json::from_value::<FuturesEvents>(value) {
            match events {
                FuturesEvents::Vec(v) => Some(FuturesWebsocketEvent::DayTickerAll(v)),
                FuturesEvents::DayTickerEvent(v) => Some(FuturesWebsocketEvent::DayTicker(v)),
                FuturesEvents::BookTickerEvent(v) => Some(FuturesWebsocketEvent::BookTicker(v)),
                FuturesEvents::MiniTickerEvent(v) => Some(FuturesWebsocketEvent::MiniTicker(v)),
                FuturesEvents::VecMiniTickerEvent(v) => Some(FuturesWebsocketEvent::MiniTickerAll(v)),
                FuturesEvents::BalanceUpdateEvent(v) => Some(FuturesWebsocketEvent::BalanceUpdateEvent(v)),
                FuturesEvents::AccountUpdateEvent(v) => Some(FuturesWebsocketEvent::AccountUpdate(v)),
                FuturesEvents::OrderTradeEvent(v) => Some(FuturesWebsocketEvent::OrderTrade(v)),
                FuturesEvents::IndexPriceEvent(v) => Some(FuturesWebsocketEvent::IndexPrice(v)),
                FuturesEvents::MarkPriceEvent(v) => Some(FuturesWebsocketEvent::MarkPrice(v)),
                FuturesEvents::VecMarkPriceEvent(v) => Some(FuturesWebsocketEvent::MarkPriceAll(v)),
                FuturesEvents::TradeEvent(v) => Some(FuturesWebsocketEvent::Trade(v)),
                FuturesEvents::ContinuousKlineEvent(v) => Some(FuturesWebsocketEvent::ContinuousKline(v)),
                FuturesEvents::IndexKlineEvent(v) => Some(FuturesWebsocketEvent::IndexKline(v)),
                FuturesEvents::LiquidationEvent(v) => Some(FuturesWebsocketEvent::Liquidation(v)),
                FuturesEvents::KlineEvent(v) => Some(FuturesWebsocketEvent::Kline(v)),
                FuturesEvents::OrderBook(v) => Some(FuturesWebsocketEvent::OrderBook(v)),
                FuturesEvents::DepthOrderBookEvent(v) => Some(FuturesWebsocketEvent::DepthOrderBook(v)),
                FuturesEvents::AggrTradesEvent(v) => Some(FuturesWebsocketEvent::AggrTrades(v)),
                FuturesEvents::UserDataStreamExpiredEvent(v) => Some(FuturesWebsocketEvent::UserDataStreamExpiredEvent(v)),
            }
        } else {
            None
        }
    }
    

    pub async fn event_loop(&mut self, running: &AtomicBool, evt_tx: mpsc::Sender<FuturesWebsocketEvent>) -> Result<(), BinanceError> {
        while running.load(Ordering::Relaxed) {
            if let Some(ref mut socket) = self.socket {
                if let Some(message) = socket.0.next().await {
                    match message {
                        Ok(Message::Text(msg)) => {
                            
                            let value: serde_json::Value = serde_json::from_str(&msg)?;
                            if let Some(data) = value.get("data") {
                                if let Some(evt) = Self::compose_event(data.clone()) {
                                    match evt_tx.send(evt).await {
                                        Ok(_) => (),
                                        Err(e) => bail!(format!("Error on sending message, {}", e)),
                                    };
                                } else {
                                    println!("Unknown event: {:?}", msg);
                                }
                            } else if let Some(evt) = Self::compose_event(value) {
                                if let FuturesWebsocketEvent::UserDataStreamExpiredEvent(e)  = evt {
                                    bail!(format!("User data stream expired:{:#?}", e));
                                }
                                match evt_tx.send(evt).await {
                                    Ok(_) => (),
                                    Err(e) => bail!(format!("Error on sending message, {}", e)),
                                };
                            } else {
                                println!("Unknown event: {:?}", msg);
                            }
                        }
                        Ok(Message::Ping(ping_data)) => {
                            socket.0.send(Message::Pong(ping_data)).await.unwrap();
                        }
                        Ok(Message::Pong(_)) | Ok(Message::Binary(_)) | Ok(Message::Frame(_)) => (),
                        Ok(Message::Close(e)) => bail!(format!("Disconnected {:?}", e)),
                        Err(e) => {
                            bail!(format!("Error on receiving message, {}", e));
                        },
                    }
                }
            }
        }
        bail!("running loop closed");
    }


}

