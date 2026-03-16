use crate::messages::{Tick, Order, OrderBook};
use rkyv::rancor::Error as RkyvError;
use std::hint::black_box;

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    rkyv::to_bytes::<RkyvError>(tick)
        .expect("rkyv encode tick")
        .into_vec()
}

/// Traditional decode: materializes owned Tick. Used for correctness checks.
pub fn decode_tick(bytes: &[u8]) -> Tick {
    rkyv::from_bytes::<Tick, RkyvError>(bytes)
        .expect("rkyv decode tick")
}

/// Zero-copy access: validates buffer and traverses all fields via archived
/// view without allocating an owned Tick (Section V-C, Table IV: zero-copy access model).
/// Returns instrument_id for optional correctness spot-checks.
pub fn access_tick(bytes: &[u8]) -> u64 {
    let archived = rkyv::access::<rkyv::Archived<Tick>, RkyvError>(bytes)
        .expect("rkyv access tick");
    black_box(archived.instrument_id);
    black_box(archived.exchange_ts_ns);
    black_box(archived.ingest_ts_ns);
    black_box(archived.seq_num);
    black_box(archived.price);
    black_box(archived.quantity);
    black_box(&archived.side);
    black_box(archived.trade_id);
    archived.instrument_id.into()
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

/// Zero-copy access for Order: traverses all fields including variable-length
/// strings via archived references (no String allocation).
pub fn access_order(bytes: &[u8]) -> u64 {
    let archived = rkyv::access::<rkyv::Archived<Order>, RkyvError>(bytes)
        .expect("rkyv access order");
    black_box(archived.instrument_id);
    black_box(archived.symbol.as_str());
    black_box(archived.order_id);
    black_box(archived.client_order_id.as_str());
    black_box(archived.client_ts_ns);
    black_box(&archived.side);
    black_box(&archived.order_type);
    black_box(archived.price);
    black_box(archived.quantity);
    archived.order_id.into()
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

/// Zero-copy access for OrderBook: traverses header fields and iterates
/// all bid/ask PriceLevels via archived slice references (no Vec allocation).
pub fn access_order_book(bytes: &[u8]) -> u64 {
    let archived = rkyv::access::<rkyv::Archived<OrderBook>, RkyvError>(bytes)
        .expect("rkyv access order_book");
    black_box(archived.instrument_id);
    black_box(archived.exchange_ts_ns);
    black_box(archived.ingest_ts_ns);
    black_box(archived.seq_num);
    for level in archived.bids.iter() {
        black_box(level.price);
        black_box(level.quantity);
    }
    for level in archived.asks.iter() {
        black_box(level.price);
        black_box(level.quantity);
    }
    archived.instrument_id.into()
}
