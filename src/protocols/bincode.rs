use crate::messages::{Tick, Order, OrderBook};

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    bincode_next::serde::encode_to_vec(tick, bincode_next::config::standard()).unwrap()
}

pub fn decode_tick(bytes: &[u8]) -> Tick {
    bincode_next::serde::decode_from_slice(bytes, bincode_next::config::standard()).unwrap().0
}

pub fn encode_order(order: &Order) -> Vec<u8> {
    bincode_next::serde::encode_to_vec(order, bincode_next::config::standard()).unwrap()
}

pub fn decode_order(bytes: &[u8]) -> Order {
    bincode_next::serde::decode_from_slice(bytes, bincode_next::config::standard()).unwrap().0
}

pub fn encode_order_book(book: &OrderBook) -> Vec<u8> {
    bincode_next::serde::encode_to_vec(book, bincode_next::config::standard()).unwrap()
}

pub fn decode_order_book(bytes: &[u8]) -> OrderBook {
    bincode_next::serde::decode_from_slice(bytes, bincode_next::config::standard()).unwrap().0
}