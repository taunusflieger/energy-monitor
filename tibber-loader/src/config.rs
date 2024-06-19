use crate::errors::TibberLoaderError;
use anyhow::Result;

pub struct Config {
    pub token: String,
    pub url: String,
}

impl Config {
    pub fn new(url: &str) -> Result<Self> {
        match std::env::var("TIBBER_API_TOKEN") {
            Ok(token) => {
                let config = Self {
                    token,
                    url: url.to_string(),
                };
                Ok(config)
            }
            _ => Err(TibberLoaderError::TokenMissing.into()),
        }
    }
}
