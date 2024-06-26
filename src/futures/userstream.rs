use crate::model::{Success, UserDataStream};
use crate::client::Client;
use crate::errors::BinanceError;
use crate::api::API;
use crate::api::Futures;

#[derive(Clone)]
pub struct FuturesUserStream {
    pub client: Client,
    pub recv_window: u64,
}

impl FuturesUserStream {
    // User Stream
    pub fn start(&self) -> Result<UserDataStream, BinanceError> {
        self.client.post(API::Futures(Futures::UserDataStream))
    }

    pub fn keep_alive(&self, listen_key: &str) -> Result<Success, BinanceError> {
        self.client
            .put(API::Futures(Futures::UserDataStream), listen_key)
    }

    pub fn close(&self, listen_key: &str) -> Result<Success, BinanceError> {
        self.client
            .delete(API::Futures(Futures::UserDataStream), listen_key)
    }
}
