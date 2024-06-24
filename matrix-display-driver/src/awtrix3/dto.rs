use serde::{Deserialize, Serialize};

#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct CustomApplication {
    pub text: String,
    #[serde(rename(serialize = "textCase"))]
    pub text_case: Option<i32>,
    #[serde(rename(serialize = "topText"))]
    pub top_text: Option<bool>,
    #[serde(rename(serialize = "textOffset"))]
    pub text_offset: Option<i32>,
    pub center: Option<bool>,
    pub color: Option<String>, // ToDo: check for better color support
    pub gradient: Option<String>,
    #[serde(rename(serialize = "blinkText"))]
    pub blink_text: Option<i32>,
    #[serde(rename(serialize = "fadeText"))]
    pub fade_text: Option<i32>,
    pub background: Option<String>, // ToDo: check for better color support
    pub rainbow: Option<bool>,
    pub icon: Option<String>,
    #[serde(rename(serialize = "pushIcon"))]
    pub push_icon: Option<i32>,
    pub repeat: Option<i32>,
    pub duration: Option<i32>,
    #[serde(rename(serialize = "lifeTime"))]
    pub life_time: Option<i32>,
}
