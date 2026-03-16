use crate::messages::{Order, OrderBook, Tick};
use std::hint::black_box;

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    serde_json::to_vec(tick).unwrap()
}

pub fn decode_tick(bytes: &[u8]) -> Tick {
    serde_json::from_slice(bytes).unwrap()
}

/// Traditional access: full deserialization (JSON has no zero-copy path).
pub fn access_tick(bytes: &[u8]) -> u64 {
    let t = decode_tick(bytes);
    black_box(&t);
    t.instrument_id
}

pub fn encode_order(order: &Order) -> Vec<u8> {
    serde_json::to_vec(order).unwrap()
}

pub fn decode_order(bytes: &[u8]) -> Order {
    serde_json::from_slice(bytes).unwrap()
}

pub fn access_order(bytes: &[u8]) -> u64 {
    let o = decode_order(bytes);
    black_box(&o);
    o.order_id
}

pub fn encode_order_book(book: &OrderBook) -> Vec<u8> {
    serde_json::to_vec(book).unwrap()
}

pub fn decode_order_book(bytes: &[u8]) -> OrderBook {
    serde_json::from_slice(bytes).unwrap()
}

pub fn access_order_book(bytes: &[u8]) -> u64 {
    let b = decode_order_book(bytes);
    black_box(&b);
    b.instrument_id
}
