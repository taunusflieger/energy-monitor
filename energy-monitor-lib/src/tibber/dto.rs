use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct PriceInformation {
    pub total: f32,
    pub level: PriceLevel,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum PriceLevel {
    Cheap,
    Expensive,
    Normal,
    VeryCheap,
    VeryExpensive,
    None,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Consumption {
    pub consumption: i32,
}
