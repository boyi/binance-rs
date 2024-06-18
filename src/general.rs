use crate::errors::BinanceError;
use crate::model::{Empty, ExchangeInformation, ServerTime, Symbol};
use crate::client::Client;
use crate::api::API;
use crate::bail;
use crate::api::Spot;

#[derive(Clone)]
pub struct General {
    pub client: Client,
}

impl General {
    // Test connectivity
    pub async fn ping(&self) -> Result<String, BinanceError> {
        self.client.get::<Empty>(API::Spot(Spot::Ping), None).await?;
        Ok("pong".into())
    }

    // Check server time
    pub async fn get_server_time(&self) -> Result<ServerTime, BinanceError> {
        self.client.get(API::Spot(Spot::Time), None).await
    }

    // Obtain exchange information
    // - Current exchange trading rules and symbol information
    pub async fn exchange_info(&self) -> Result<ExchangeInformation, BinanceError> {
        self.client.get(API::Spot(Spot::ExchangeInfo), None).await
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
