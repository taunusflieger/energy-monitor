use crate::awtrix3::dto::*;
use energy_monitor_lib::topic::Topic;

#[rustfmt::skip]

pub const MATRIX_DISPLAY_APP_YIELD_DAY_TOPIC: Topic<CustomApplication> = Topic::new("matrixdisplay/custom/yieldday");
pub const MATRIX_DISPLAY_APP_CURRENT_PRODUCTION_TOPIC: Topic<CustomApplication> =
    Topic::new("matrixdisplay/custom/power");
pub const MATRIX_DISPLAY_APP_CURRENT_CONSUMPTION_TOPIC: Topic<CustomApplication> =
    Topic::new("matrixdisplay/custom/consumption");
pub const MATRIX_DISPLAY_APP_CURRENT_PRICE_TOPIC: Topic<CustomApplication> =
    Topic::new("matrixdisplay/custom/tibberprice");
