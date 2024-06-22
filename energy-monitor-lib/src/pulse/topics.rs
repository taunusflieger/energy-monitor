use crate::pulse::dto::*;
use crate::topic::Topic;
#[rustfmt::skip]

pub const PULSE_CONSUMPTION_TOPIC: Topic<Consumption> = Topic::new("Pulse/consumption");
