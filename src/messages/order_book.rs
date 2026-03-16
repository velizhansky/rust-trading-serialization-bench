use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[repr(C)]
pub struct PriceLevel {
    pub price: i64,
    pub quantity: i64,
}

#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
pub struct OrderBook {
    pub instrument_id: u64,
    pub exchange_ts_ns: u64,
    pub ingest_ts_ns: u64,
    pub seq_num: u64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}
