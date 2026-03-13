use crate::messages::{Tick, Order, OrderBook, Side, OrderType};
use prost::Message;
#[derive(Clone, PartialEq, Message)]
pub struct ProtoTick {
    #[prost(uint64, tag = "1")]
    pub instrument_id: u64,
    #[prost(uint64, tag = "2")]
    pub exchange_ts_ns: u64,
    #[prost(uint64, tag = "3")]
    pub ingest_ts_ns: u64,
    #[prost(uint64, tag = "4")]
    pub seq_num: u64,
    #[prost(int64, tag = "5")]
    pub price: i64,
    #[prost(int64, tag = "6")]
    pub quantity: i64,
    #[prost(enumeration = "ProtoSide", tag = "7")]
    pub side: i32,
    #[prost(uint64, tag = "8")]
    pub trade_id: u64,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoOrder {
    #[prost(uint64, tag = "1")]
    pub instrument_id: u64,
    #[prost(string, tag = "2")]
    pub symbol: String,
    #[prost(uint64, tag = "3")]
    pub order_id: u64,
    #[prost(string, tag = "4")]
    pub client_order_id: String,
    #[prost(uint64, tag = "5")]
    pub client_ts_ns: u64,
    #[prost(enumeration = "ProtoSide", tag = "6")]
    pub side: i32,
    #[prost(enumeration = "ProtoOrderType", tag = "7")]
    pub order_type: i32,
    #[prost(int64, tag = "8")]
    pub price: i64,
    #[prost(int64, tag = "9")]
    pub quantity: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoPriceLevel {
    #[prost(int64, tag = "1")]
    pub price: i64,
    #[prost(int64, tag = "2")]
    pub quantity: i64,
}

#[derive(Clone, PartialEq, Message)]
pub struct ProtoOrderBook {
    #[prost(uint64, tag = "1")]
    pub instrument_id: u64,
    #[prost(uint64, tag = "2")]
    pub exchange_ts_ns: u64,
    #[prost(uint64, tag = "3")]
    pub ingest_ts_ns: u64,
    #[prost(uint64, tag = "4")]
    pub seq_num: u64,
    #[prost(message, repeated, tag = "5")]
    pub bids: Vec<ProtoPriceLevel>,
    #[prost(message, repeated, tag = "6")]
    pub asks: Vec<ProtoPriceLevel>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum ProtoSide {
    Unspecified = 0,
    Buy = 1,
    Sell = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum ProtoOrderType {
    Unspecified = 0,
    Limit = 1,
    Market = 2,
}

// Conversion functions
fn to_proto_side(side: Side) -> i32 {
    match side {
        Side::Unspecified => ProtoSide::Unspecified as i32,
        Side::Buy => ProtoSide::Buy as i32,
        Side::Sell => ProtoSide::Sell as i32,
    }
}

fn from_proto_side(side: i32) -> Side {
    match ProtoSide::try_from(side).unwrap_or(ProtoSide::Unspecified) {
        ProtoSide::Unspecified => Side::Unspecified,
        ProtoSide::Buy => Side::Buy,
        ProtoSide::Sell => Side::Sell,
    }
}

fn to_proto_order_type(order_type: OrderType) -> i32 {
    match order_type {
        OrderType::Unspecified => ProtoOrderType::Unspecified as i32,
        OrderType::Limit => ProtoOrderType::Limit as i32,
        OrderType::Market => ProtoOrderType::Market as i32,
    }
}

fn from_proto_order_type(order_type: i32) -> OrderType {
    match ProtoOrderType::try_from(order_type).unwrap_or(ProtoOrderType::Unspecified) {
        ProtoOrderType::Unspecified => OrderType::Unspecified,
        ProtoOrderType::Limit => OrderType::Limit,
        ProtoOrderType::Market => OrderType::Market,
    }
}

pub fn encode_tick(tick: &Tick) -> Vec<u8> {
    let proto = ProtoTick {
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
    let proto = ProtoTick::decode(bytes).expect("Failed to decode Tick");
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
    let proto = ProtoOrder {
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
    let proto = ProtoOrder::decode(bytes).expect("Failed to decode Order");
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
    let proto = ProtoOrderBook {
        instrument_id: book.instrument_id,
        exchange_ts_ns: book.exchange_ts_ns,
        ingest_ts_ns: book.ingest_ts_ns,
        seq_num: book.seq_num,
        bids: book.bids.iter().map(|level| ProtoPriceLevel {
            price: level.price,
            quantity: level.quantity,
        }).collect(),
        asks: book.asks.iter().map(|level| ProtoPriceLevel {
            price: level.price,
            quantity: level.quantity,
        }).collect(),
    };
    proto.encode_to_vec()
}

pub fn decode_order_book(bytes: &[u8]) -> OrderBook {
    let proto = ProtoOrderBook::decode(bytes).expect("Failed to decode OrderBook");
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

