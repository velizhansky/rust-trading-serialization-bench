use crate::messages::{Tick, Order, OrderBook};

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    serde_json::to_vec(tick).unwrap()
}

pub fn decode_tick(bytes: &[u8]) -> Tick {
    serde_json::from_slice(bytes).unwrap()
}

pub fn encode_order(order: &Order) -> Vec<u8> {
    serde_json::to_vec(order).unwrap()
}

pub fn decode_order(bytes: &[u8]) -> Order {
    serde_json::from_slice(bytes).unwrap()
}

pub fn encode_order_book(book: &OrderBook) -> Vec<u8> {
    serde_json::to_vec(book).unwrap()
}

pub fn decode_order_book(bytes: &[u8]) -> OrderBook {
    serde_json::from_slice(bytes).unwrap()
}