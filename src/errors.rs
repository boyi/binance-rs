use thiserror::Error;

use serde::Deserialize;
// use error_chain::error_chain;

#[derive(Debug, Deserialize)]
pub struct BinanceContentError {
    pub code: i16,
    pub msg: String,
}

#[derive(Error, Debug)]
pub enum BinanceError {
    #[error("binance content error: ({code:?}, {msg:?})")]
    BinanceError{code: i16, msg: String},
    #[error("invalid Vec for Kline ({name:?} at index {index:?} is missing)")]
    KlineValueMissingError { index: usize, name: &'static str },
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
    Tungstenite(#[from] tungstenite::Error),
    #[error(transparent)]
    TimestampError(#[from] std::time::SystemTimeError),
    #[error("binance error: {0}")]
    OtherError(String),
}

#[macro_export]
macro_rules! bail {
    ($e:expr) => {
        return Err(BinanceError::OtherError($e.to_string()))
    };
}

// error_chain! {
//     errors {
//         BinanceError(response: BinanceContentError)

//         KlineValueMissingError(index: usize, name: &'static str) {
//             description("invalid Vec for Kline"),
//             display("{} at {} is missing", name, index),
//         }
//      }

//     foreign_links {
//         ReqError(reqwest::Error);
//         InvalidHeaderError(reqwest::header::InvalidHeaderValue);
//         IoError(std::io::Error);
//         ParseFloatError(std::num::ParseFloatError);
//         UrlParserError(url::ParseError);
//         Json(serde_json::Error);
//         Tungstenite(tungstenite::Error);
//         TimestampError(std::time::SystemTimeError);
//     }
// }
