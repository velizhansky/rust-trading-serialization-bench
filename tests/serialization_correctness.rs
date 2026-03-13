use rust_trading_serialization_bench::messages::{Tick, Order, OrderBook, PriceLevel, Side, OrderType};
use rust_trading_serialization_bench::protocols;

#[test]
fn test_tick_json_roundtrip() {
    let tick = Tick {
        instrument_id: 1,
        exchange_ts_ns: 100,
        ingest_ts_ns: 200,
        seq_num: 1,
        price: 10000,
        quantity: 100,
        side: Side::Buy,
        trade_id: 12345,
    };
    
    let json = serde_json::to_vec(&tick).unwrap();
    let tick_back: Tick = serde_json::from_slice(&json).unwrap();
    assert_eq!(tick, tick_back);
}

#[test]
fn test_tick_bincode_roundtrip() {
    let tick = Tick {
        instrument_id: 1,
        exchange_ts_ns: 100,
        ingest_ts_ns: 200,
        seq_num: 1,
        price: 10000,
        quantity: 100,
        side: Side::Buy,
        trade_id: 12345,
    };
    
    let bytes = bincode_next::serde::encode_to_vec(&tick, bincode_next::config::standard()).unwrap();
    let tick_back: Tick = bincode_next::serde::decode_from_slice(&bytes, bincode_next::config::standard()).unwrap().0;
    assert_eq!(tick, tick_back);
}

#[test]
fn test_tick_rkyv_roundtrip() {
    let tick = Tick {
        instrument_id: 1,
        exchange_ts_ns: 100,
        ingest_ts_ns: 200,
        seq_num: 1,
        price: 10000,
        quantity: 100,
        side: Side::Buy,
        trade_id: 12345,
    };
    
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&tick).unwrap();
    let tick_back: Tick = rkyv::from_bytes::<Tick, rkyv::rancor::Error>(&bytes).unwrap();
    assert_eq!(tick, tick_back);
}

#[test]
fn test_order_json_roundtrip() {
    let order = Order {
        instrument_id: 1,
        symbol: "BTCUSD".to_string(),
        order_id: 123,
        client_order_id: "client123".to_string(),
        client_ts_ns: 1000,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: 50000,
        quantity: 1,
    };
    
    let json = serde_json::to_vec(&order).unwrap();
    let order_back: Order = serde_json::from_slice(&json).unwrap();
    assert_eq!(order, order_back);
}

#[test]
fn test_order_rkyv_roundtrip() {
    let order = Order {
        instrument_id: 1,
        symbol: "BTCUSD".to_string(),
        order_id: 123,
        client_order_id: "client123".to_string(),
        client_ts_ns: 1000,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: 50000,
        quantity: 1,
    };
    
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&order).unwrap();
    let order_back: Order = rkyv::from_bytes::<Order, rkyv::rancor::Error>(&bytes).unwrap();
    assert_eq!(order, order_back);
}

#[test]
fn test_order_book_json_roundtrip() {
    let book = OrderBook {
        instrument_id: 1,
        exchange_ts_ns: 1000,
        ingest_ts_ns: 2000,
        seq_num: 1,
        bids: vec![
            PriceLevel { price: 50000, quantity: 10 },
            PriceLevel { price: 49990, quantity: 20 },
        ],
        asks: vec![
            PriceLevel { price: 50010, quantity: 15 },
            PriceLevel { price: 50020, quantity: 25 },
        ],
    };
    
    let json = serde_json::to_vec(&book).unwrap();
    let book_back: OrderBook = serde_json::from_slice(&json).unwrap();
    assert_eq!(book, book_back);
}

#[test]
fn test_order_book_rkyv_roundtrip() {
    let book = OrderBook {
        instrument_id: 1,
        exchange_ts_ns: 1000,
        ingest_ts_ns: 2000,
        seq_num: 1,
        bids: vec![
            PriceLevel { price: 50000, quantity: 10 },
            PriceLevel { price: 49990, quantity: 20 },
        ],
        asks: vec![
            PriceLevel { price: 50010, quantity: 15 },
            PriceLevel { price: 50020, quantity: 25 },
        ],
    };
    
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&book).unwrap();
    let book_back: OrderBook = rkyv::from_bytes::<OrderBook, rkyv::rancor::Error>(&bytes).unwrap();
    assert_eq!(book, book_back);
}

#[test]
fn test_tick_protobuf_roundtrip() {
    let tick = Tick {
        instrument_id: 1,
        exchange_ts_ns: 100,
        ingest_ts_ns: 200,
        seq_num: 1,
        price: 10000,
        quantity: 100,
        side: Side::Buy,
        trade_id: 12345,
    };
    
    let bytes = protocols::protobuf::encode_tick(&tick);
    let tick_back = protocols::protobuf::decode_tick(&bytes);
    assert_eq!(tick, tick_back);
}

#[test]
fn test_order_protobuf_roundtrip() {
    let order = Order {
        instrument_id: 1,
        symbol: "BTCUSD".to_string(),
        order_id: 123,
        client_order_id: "client123".to_string(),
        client_ts_ns: 1000,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: 50000,
        quantity: 1,
    };
    
    let bytes = protocols::protobuf::encode_order(&order);
    let order_back = protocols::protobuf::decode_order(&bytes);
    assert_eq!(order, order_back);
}

#[test]
fn test_order_book_protobuf_roundtrip() {
    let book = OrderBook {
        instrument_id: 1,
        exchange_ts_ns: 1000,
        ingest_ts_ns: 2000,
        seq_num: 1,
        bids: vec![
            PriceLevel { price: 50000, quantity: 10 },
            PriceLevel { price: 49990, quantity: 20 },
        ],
        asks: vec![
            PriceLevel { price: 50010, quantity: 15 },
            PriceLevel { price: 50020, quantity: 25 },
        ],
    };
    
    let bytes = protocols::protobuf::encode_order_book(&book);
    let book_back = protocols::protobuf::decode_order_book(&bytes);
    assert_eq!(book, book_back);
}

#[test]
fn test_tick_flatbuffers_roundtrip() {
    let tick = Tick {
        instrument_id: 1,
        exchange_ts_ns: 100,
        ingest_ts_ns: 200,
        seq_num: 1,
        price: 10000,
        quantity: 100,
        side: Side::Buy,
        trade_id: 12345,
    };
    
    let bytes = protocols::flatbuffers::encode_tick(&tick);
    let tick_back = protocols::flatbuffers::decode_tick(&bytes);
    assert_eq!(tick, tick_back);
}

#[test]
fn test_order_flatbuffers_roundtrip() {
    let order = Order {
        instrument_id: 1,
        symbol: "BTCUSD".to_string(),
        order_id: 123,
        client_order_id: "client123".to_string(),
        client_ts_ns: 1000,
        side: Side::Buy,
        order_type: OrderType::Limit,
        price: 50000,
        quantity: 1,
    };
    
    let bytes = protocols::flatbuffers::encode_order(&order);
    let order_back = protocols::flatbuffers::decode_order(&bytes);
    assert_eq!(order, order_back);
}

#[test]
fn test_order_book_flatbuffers_roundtrip() {
    let book = OrderBook {
        instrument_id: 1,
        exchange_ts_ns: 1000,
        ingest_ts_ns: 2000,
        seq_num: 1,
        bids: vec![
            PriceLevel { price: 50000, quantity: 10 },
            PriceLevel { price: 49990, quantity: 20 },
        ],
        asks: vec![
            PriceLevel { price: 50010, quantity: 15 },
            PriceLevel { price: 50020, quantity: 25 },
        ],
    };
    
    let bytes = protocols::flatbuffers::encode_order_book(&book);
    let book_back = protocols::flatbuffers::decode_order_book(&bytes);
    assert_eq!(book, book_back);
}

