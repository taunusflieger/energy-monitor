use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PriceInformation {
    pub total: f32,
    pub level: PriceLevel,
}

#[derive(Serialize, Deserialize)]
pub enum PriceLevel {
    Cheap,
    Expensive,
    Normal,
    VeryCheap,
    VeryExpensive,
    None,
}
