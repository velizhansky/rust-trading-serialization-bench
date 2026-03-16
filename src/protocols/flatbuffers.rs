use crate::messages::{Tick, Order, OrderBook, PriceLevel, Side, OrderType};
use std::hint::black_box;

#[allow(dead_code, unused_imports, non_snake_case, clippy::all)]
mod trading_generated {
    include!(concat!(env!("OUT_DIR"), "/trading_generated.rs"));
}

use trading_generated::trading_fbs;
fn to_fb_side(side: Side) -> trading_fbs::Side {
    match side {
        Side::Unspecified => trading_fbs::Side::Unspecified,
        Side::Buy => trading_fbs::Side::Buy,
        Side::Sell => trading_fbs::Side::Sell,
    }
}

fn from_fb_side(side: trading_fbs::Side) -> Side {
    match side {
        trading_fbs::Side::Buy => Side::Buy,
        trading_fbs::Side::Sell => Side::Sell,
        _ => Side::Unspecified,
    }
}

fn to_fb_order_type(order_type: OrderType) -> trading_fbs::OrderType {
    match order_type {
        OrderType::Unspecified => trading_fbs::OrderType::Unspecified,
        OrderType::Limit => trading_fbs::OrderType::Limit,
        OrderType::Market => trading_fbs::OrderType::Market,
    }
}

fn from_fb_order_type(order_type: trading_fbs::OrderType) -> OrderType {
    match order_type {
        trading_fbs::OrderType::Limit => OrderType::Limit,
        trading_fbs::OrderType::Market => OrderType::Market,
        _ => OrderType::Unspecified,
    }
}

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(256);

    let fb_tick = trading_fbs::Tick::create(&mut builder, &trading_fbs::TickArgs {
        instrument_id: tick.instrument_id,
        exchange_ts_ns: tick.exchange_ts_ns,
        ingest_ts_ns: tick.ingest_ts_ns,
        seq_num: tick.seq_num,
        price: tick.price,
        quantity: tick.quantity,
        side: to_fb_side(tick.side),
        trade_id: tick.trade_id,
    });

    builder.finish(fb_tick, None);
    builder.finished_data().to_vec()
}

/// Traditional decode: materializes owned Tick. Used for correctness checks.
pub fn decode_tick(bytes: &[u8]) -> Tick {
    let fb_tick = flatbuffers::root::<trading_fbs::Tick>(bytes)
        .expect("Invalid FlatBuffers data");

    Tick {
        instrument_id: fb_tick.instrument_id(),
        exchange_ts_ns: fb_tick.exchange_ts_ns(),
        ingest_ts_ns: fb_tick.ingest_ts_ns(),
        seq_num: fb_tick.seq_num(),
        price: fb_tick.price(),
        quantity: fb_tick.quantity(),
        side: from_fb_side(fb_tick.side()),
        trade_id: fb_tick.trade_id(),
    }
}

/// Zero-copy access: parses buffer and reads all fields via FlatBuffers
/// accessor methods without allocating an owned Tick (Section V-C, Table IV).
pub fn access_tick(bytes: &[u8]) -> u64 {
    let fb = flatbuffers::root::<trading_fbs::Tick>(bytes)
        .expect("Invalid FlatBuffers data");
    black_box(fb.instrument_id());
    black_box(fb.exchange_ts_ns());
    black_box(fb.ingest_ts_ns());
    black_box(fb.seq_num());
    black_box(fb.price());
    black_box(fb.quantity());
    black_box(fb.side());
    black_box(fb.trade_id());
    fb.instrument_id()
}

pub fn encode_order(order: &Order) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(1024);

    let symbol = builder.create_string(&order.symbol);
    let client_order_id = builder.create_string(&order.client_order_id);

    let fb_order = trading_fbs::Order::create(&mut builder, &trading_fbs::OrderArgs {
        instrument_id: order.instrument_id,
        symbol: Some(symbol),
        order_id: order.order_id,
        client_order_id: Some(client_order_id),
        client_ts_ns: order.client_ts_ns,
        side: to_fb_side(order.side),
        order_type: to_fb_order_type(order.order_type),
        price: order.price,
        quantity: order.quantity,
    });

    builder.finish(fb_order, None);
    builder.finished_data().to_vec()
}

pub fn decode_order(bytes: &[u8]) -> Order {
    let fb_order = flatbuffers::root::<trading_fbs::Order>(bytes)
        .expect("Invalid FlatBuffers data");

    Order {
        instrument_id: fb_order.instrument_id(),
        symbol: fb_order.symbol().unwrap_or("").to_string(),
        order_id: fb_order.order_id(),
        client_order_id: fb_order.client_order_id().unwrap_or("").to_string(),
        client_ts_ns: fb_order.client_ts_ns(),
        side: from_fb_side(fb_order.side()),
        order_type: from_fb_order_type(fb_order.order_type()),
        price: fb_order.price(),
        quantity: fb_order.quantity(),
    }
}

/// Zero-copy access for Order: reads all fields including strings via
/// FlatBuffers &str references (no String allocation).
pub fn access_order(bytes: &[u8]) -> u64 {
    let fb = flatbuffers::root::<trading_fbs::Order>(bytes)
        .expect("Invalid FlatBuffers data");
    black_box(fb.instrument_id());
    black_box(fb.symbol());
    black_box(fb.order_id());
    black_box(fb.client_order_id());
    black_box(fb.client_ts_ns());
    black_box(fb.side());
    black_box(fb.order_type());
    black_box(fb.price());
    black_box(fb.quantity());
    fb.order_id()
}

pub fn encode_order_book(book: &OrderBook) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(4096);

    let bids: Vec<_> = book.bids.iter()
        .map(|level| trading_fbs::PriceLevel::new(level.price, level.quantity))
        .collect();
    let bids_vec = builder.create_vector(&bids);

    let asks: Vec<_> = book.asks.iter()
        .map(|level| trading_fbs::PriceLevel::new(level.price, level.quantity))
        .collect();
    let asks_vec = builder.create_vector(&asks);

    let fb_book = trading_fbs::OrderBook::create(&mut builder, &trading_fbs::OrderBookArgs {
        instrument_id: book.instrument_id,
        exchange_ts_ns: book.exchange_ts_ns,
        ingest_ts_ns: book.ingest_ts_ns,
        seq_num: book.seq_num,
        bids: Some(bids_vec),
        asks: Some(asks_vec),
    });

    builder.finish(fb_book, None);
    builder.finished_data().to_vec()
}

pub fn decode_order_book(bytes: &[u8]) -> OrderBook {
    let fb_book = flatbuffers::root::<trading_fbs::OrderBook>(bytes)
        .expect("Invalid FlatBuffers data");

    let bids = fb_book.bids()
        .map(|levels| {
            levels.iter()
                .map(|level| PriceLevel {
                    price: level.price(),
                    quantity: level.quantity(),
                })
                .collect()
        })
        .unwrap_or_default();

    let asks = fb_book.asks()
        .map(|levels| {
            levels.iter()
                .map(|level| PriceLevel {
                    price: level.price(),
                    quantity: level.quantity(),
                })
                .collect()
        })
        .unwrap_or_default();

    OrderBook {
        instrument_id: fb_book.instrument_id(),
        exchange_ts_ns: fb_book.exchange_ts_ns(),
        ingest_ts_ns: fb_book.ingest_ts_ns(),
        seq_num: fb_book.seq_num(),
        bids,
        asks,
    }
}

/// Zero-copy access for OrderBook: reads header fields and iterates all
/// bid/ask PriceLevels via FlatBuffers vector accessors (no Vec allocation).
pub fn access_order_book(bytes: &[u8]) -> u64 {
    let fb = flatbuffers::root::<trading_fbs::OrderBook>(bytes)
        .expect("Invalid FlatBuffers data");
    black_box(fb.instrument_id());
    black_box(fb.exchange_ts_ns());
    black_box(fb.ingest_ts_ns());
    black_box(fb.seq_num());
    if let Some(bids) = fb.bids() {
        for level in bids.iter() {
            black_box(level.price());
            black_box(level.quantity());
        }
    }
    if let Some(asks) = fb.asks() {
        for level in asks.iter() {
            black_box(level.price());
            black_box(level.quantity());
        }
    }
    fb.instrument_id()
}
