
use crate::model::Empty;
use crate::futures::model::{ExchangeInformation, ServerTime, Symbol};
use crate::client::Client;
use crate::errors::BinanceError;
use crate::api::API;
use crate::api::Futures;
use crate::bail;

#[derive(Clone)]
pub struct FuturesGeneral {
    pub client: Client,
}

impl FuturesGeneral {
    // Test connectivity
    pub async fn ping(&self) -> Result<String, BinanceError> {
        self.client.get::<Empty>(API::Futures(Futures::Ping), None).await?;
        Ok("pong".into())
    }

    // Check server time
    pub async fn get_server_time(&self) -> Result<ServerTime, BinanceError> {
        self.client.get(API::Futures(Futures::Time), None).await
    }

    // Obtain exchange information
    // - Current exchange trading rules and symbol information
    pub async fn exchange_info(&self) -> Result<ExchangeInformation, BinanceError> {
        self.client.get(API::Futures(Futures::ExchangeInfo), None).await
    }

    // Get Symbol information
    pub async fn get_symbol_info<S>(&self, symbol: S) -> Result<Symbol, BinanceError>
    where
        S: Into<String>,
    {
        let upper_symbol = symbol.into().to_uppercase();
        match self.exchange_info().await {
            Ok(info) => {
                for item in info.symbols {
                    if item.symbol == upper_symbol {
                        return Ok(item);
                    }
                }
                bail!("Symbol not found")
            }
            Err(e) => Err(e),
        }
    }
}
