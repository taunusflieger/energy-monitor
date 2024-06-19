use chrono::{DateTime, FixedOffset};
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/schema.json",
    query_path = "src/gql/queries/strings/view.graphql",
    response_derives = "Debug, Serialize, Clone"
)]
pub struct Viewer;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/gql/schema.json",
    query_path = "src/gql/queries/strings/price.graphql",
    response_derives = "Debug, Serialize, Clone"
)]
pub struct Price;

#[derive(Debug, Clone)]
/// Price level based on trailing price average (3 days for hourly values and 30 days for daily values)
pub enum PriceLevel {
    /// The price is smaller or equal to 60 % compared to average price.
    VeryCheap,
    /// The price is greater than 60 % and smaller or equal to 90 % compared to average price.
    Cheap,
    /// The price is greater than 90 % and smaller than 115 % compared to average price.
    Normal,
    /// The price is greater or equal to 115 % and smaller than 140 % compared to average price.
    Expensive,
    /// The price is greater or equal to 140 % compared to average price.
    VeryExpensive,
    /// Other
    Other(String),
    /// Missing data
    None,
}

#[derive(Debug, Clone)]
/// Price information related to the subscription for the current hour
pub struct PriceInfo {
    /// The total price (incl. tax)
    pub total: f64,
    /// Nord Pool spot price
    pub energy: f64,
    /// The tax part of the price (guarantee of origin certificate, energy tax (Sweden only) and VAT)
    pub tax: f64,
    /// The start time of the price
    pub starts_at: DateTime<FixedOffset>,
    /// The cost currency
    pub currency: String,
    /// The price level compared to recent price values
    pub level: PriceLevel,
}

impl PriceInfo {
    pub fn new(pinfo: price::PriceViewerHomeCurrentSubscriptionPriceInfoCurrent) -> Option<Self> {
        let total = pinfo.total?;
        let (energy, tax) = match (pinfo.energy, pinfo.tax) {
            (Some(energy), Some(tax)) => (energy, tax),
            (Some(energy), None) => (energy, total - energy),
            (None, Some(tax)) => (total - tax, tax),
            _ => (total, 0.0),
        };

        let level = match pinfo.level {
            Some(price::PriceLevel::VERY_CHEAP) => PriceLevel::VeryCheap,
            Some(price::PriceLevel::CHEAP) => PriceLevel::Cheap,
            Some(price::PriceLevel::NORMAL) => PriceLevel::Normal,
            Some(price::PriceLevel::EXPENSIVE) => PriceLevel::Expensive,
            Some(price::PriceLevel::VERY_EXPENSIVE) => PriceLevel::VeryExpensive,
            Some(price::PriceLevel::Other(s)) => PriceLevel::Other(s),
            _ => PriceLevel::None,
        };

        let starts_at = chrono::DateTime::parse_from_rfc3339(
            pinfo
                .starts_at
                .ok_or("Missing starts_at time")
                .ok()?
                .as_str(),
        )
        .ok()?;

        Some(PriceInfo {
            total,
            energy,
            tax,
            starts_at,
            currency: pinfo.currency,
            level,
        })
    }
}
