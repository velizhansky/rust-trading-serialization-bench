//! Protobuf serialization via prost, using types generated from
//! schemas/trading.proto at compile time (Section V-C, Table IV).

use crate::messages::{Tick, Order, OrderBook, Side, OrderType};
use prost::Message;

/// Generated Protobuf types from schemas/trading.proto via prost-build.
mod proto {
    include!(concat!(env!("OUT_DIR"), "/trading_proto.rs"));
}

// --- Enum conversions ---

fn to_proto_side(side: Side) -> i32 {
    match side {
        Side::Unspecified => proto::Side::Unspecified as i32,
        Side::Buy => proto::Side::Buy as i32,
        Side::Sell => proto::Side::Sell as i32,
    }
}

fn from_proto_side(side: i32) -> Side {
    match proto::Side::try_from(side).unwrap_or(proto::Side::Unspecified) {
        proto::Side::Unspecified => Side::Unspecified,
        proto::Side::Buy => Side::Buy,
        proto::Side::Sell => Side::Sell,
    }
}

fn to_proto_order_type(order_type: OrderType) -> i32 {
    match order_type {
        OrderType::Unspecified => proto::OrderType::Unspecified as i32,
        OrderType::Limit => proto::OrderType::Limit as i32,
        OrderType::Market => proto::OrderType::Market as i32,
    }
}

fn from_proto_order_type(order_type: i32) -> OrderType {
    match proto::OrderType::try_from(order_type).unwrap_or(proto::OrderType::Unspecified) {
        proto::OrderType::Unspecified => OrderType::Unspecified,
        proto::OrderType::Limit => OrderType::Limit,
        proto::OrderType::Market => OrderType::Market,
    }
}

// --- Encode/Decode ---

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    let proto = proto::Tick {
        instrument_id: tick.instrument_id,
        exchange_ts_ns: tick.exchange_ts_ns,
        ingest_ts_ns: tick.ingest_ts_ns,
        seq_num: tick.seq_num,
        price: tick.price,
        quantity: tick.quantity,
        side: to_proto_side(tick.side),
        trade_id: tick.trade_id,
    };
    proto.encode_to_vec()
}

pub fn decode_tick(bytes: &[u8]) -> Tick {
    let proto = proto::Tick::decode(bytes).expect("Failed to decode Tick");
    Tick {
        instrument_id: proto.instrument_id,
        exchange_ts_ns: proto.exchange_ts_ns,
        ingest_ts_ns: proto.ingest_ts_ns,
        seq_num: proto.seq_num,
        price: proto.price,
        quantity: proto.quantity,
        side: from_proto_side(proto.side),
        trade_id: proto.trade_id,
    }
}

pub fn encode_order(order: &Order) -> Vec<u8> {
    let proto = proto::Order {
        instrument_id: order.instrument_id,
        symbol: order.symbol.clone(),
        order_id: order.order_id,
        client_order_id: order.client_order_id.clone(),
        client_ts_ns: order.client_ts_ns,
        side: to_proto_side(order.side),
        order_type: to_proto_order_type(order.order_type),
        price: order.price,
        quantity: order.quantity,
    };
    proto.encode_to_vec()
}

pub fn decode_order(bytes: &[u8]) -> Order {
    let proto = proto::Order::decode(bytes).expect("Failed to decode Order");
    Order {
        instrument_id: proto.instrument_id,
        symbol: proto.symbol,
        order_id: proto.order_id,
        client_order_id: proto.client_order_id,
        client_ts_ns: proto.client_ts_ns,
        side: from_proto_side(proto.side),
        order_type: from_proto_order_type(proto.order_type),
        price: proto.price,
        quantity: proto.quantity,
    }
}

pub fn encode_order_book(book: &OrderBook) -> Vec<u8> {
    let proto = proto::OrderBook {
        instrument_id: book.instrument_id,
        exchange_ts_ns: book.exchange_ts_ns,
        ingest_ts_ns: book.ingest_ts_ns,
        seq_num: book.seq_num,
        bids: book.bids.iter().map(|level| proto::PriceLevel {
            price: level.price,
            quantity: level.quantity,
        }).collect(),
        asks: book.asks.iter().map(|level| proto::PriceLevel {
            price: level.price,
            quantity: level.quantity,
        }).collect(),
    };
    proto.encode_to_vec()
}

pub fn decode_order_book(bytes: &[u8]) -> OrderBook {
    let proto = proto::OrderBook::decode(bytes).expect("Failed to decode OrderBook");
    OrderBook {
        instrument_id: proto.instrument_id,
        exchange_ts_ns: proto.exchange_ts_ns,
        ingest_ts_ns: proto.ingest_ts_ns,
        seq_num: proto.seq_num,
        bids: proto.bids.iter().map(|level| crate::messages::PriceLevel {
            price: level.price,
            quantity: level.quantity,
        }).collect(),
        asks: proto.asks.iter().map(|level| crate::messages::PriceLevel {
            price: level.price,
            quantity: level.quantity,
        }).collect(),
    }
}

// --- Traditional access (Protobuf has no zero-copy path) ---

pub fn access_tick(bytes: &[u8]) -> u64 {
    let t = decode_tick(bytes);
    std::hint::black_box(&t);
    t.instrument_id
}

pub fn access_order(bytes: &[u8]) -> u64 {
    let o = decode_order(bytes);
    std::hint::black_box(&o);
    o.order_id
}

pub fn access_order_book(bytes: &[u8]) -> u64 {
    let b = decode_order_book(bytes);
    std::hint::black_box(&b);
    b.instrument_id
}
