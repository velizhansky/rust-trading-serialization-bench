use super::Side;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[repr(C)]
pub struct Tick {
    pub instrument_id: u64,
    pub exchange_ts_ns: u64,
    pub ingest_ts_ns: u64,
    pub seq_num: u64,
    pub price: i64,
    pub quantity: i64,
    pub side: Side,
    pub trade_id: u64,
}
