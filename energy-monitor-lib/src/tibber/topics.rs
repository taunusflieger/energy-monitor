use crate::tibber::dto::*;
use crate::topic::Topic;
#[rustfmt::skip]
pub const TIBBER_PRICE_INFORMATION_TOPIC: Topic<PriceInformation> =
    Topic::new("Tibber/price_information");

pub const TIBBER_CONSUMPTION_TOPIC: Topic<Consumption> = Topic::new("Tibber/consumption");
