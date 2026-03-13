use crate::messages::{Tick, Order, OrderBook};
use rkyv::rancor::Error as RkyvError;

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    rkyv::to_bytes::<RkyvError>(tick)
        .expect("rkyv encode tick")
        .into_vec()
}

pub fn decode_tick(bytes: &[u8]) -> Tick {
    rkyv::from_bytes::<Tick, RkyvError>(bytes)
        .expect("rkyv decode tick")
}

pub fn encode_order(order: &Order) -> Vec<u8> {
    rkyv::to_bytes::<RkyvError>(order)
        .expect("rkyv encode order")
        .into_vec()
}

pub fn decode_order(bytes: &[u8]) -> Order {
    rkyv::from_bytes::<Order, RkyvError>(bytes)
        .expect("rkyv decode order")
}

pub fn encode_order_book(book: &OrderBook) -> Vec<u8> {
    rkyv::to_bytes::<RkyvError>(book)
        .expect("rkyv encode order_book")
        .into_vec()
}

pub fn decode_order_book(bytes: &[u8]) -> OrderBook {
    rkyv::from_bytes::<OrderBook, RkyvError>(bytes)
        .expect("rkyv decode order_book")
}