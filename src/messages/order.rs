use super::{Side, OrderType};
use serde::{Deserialize, Serialize};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct Order {
    pub instrument_id: u64,
    pub symbol: String,
    pub order_id: u64,
    pub client_order_id: String,
    pub client_ts_ns: u64,
    pub side: Side,
    pub order_type: OrderType,
    pub price: i64,
    pub quantity: i64,
}