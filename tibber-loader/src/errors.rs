use reqwest::header::InvalidHeaderValue;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TibberLoaderError {
    #[error("Missing Tibber API token. Please set the TIBBER_API_TOKEN environmental variable")]
    TokenMissing,

    #[error("Tibber API token is not vaild invalid")]
    Unauthorized,

    #[error("Invalide http header value")]
    InvalidHeader(#[from] InvalidHeaderValue),

    #[error("Failed to get data from GraphQL response")]
    MissingResponseData,

    #[error("Only one home is supported")]
    OnlyOneHomeSupported,

    #[error("{0}")]
    GraphQLError(String),

    #[error("Failed to fetch: {0}")]
    FetchError(#[from] reqwest::Error),

    #[error("No subscription")]
    NoSubscription,

    #[error("No price info")]
    NoPriceInfo,

    #[error("No current price")]
    NoCurrentPrice,
}
