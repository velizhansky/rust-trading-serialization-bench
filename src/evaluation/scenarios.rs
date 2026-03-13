use crate::messages::{Tick, Order, OrderBook, PriceLevel, Side, OrderType};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const INSTRUMENTS: &[u64] = &[
    100001, 100002, 100003, 100004, 100005,
    200001, 200002, 200003, 200004, 200005,
    300001, 300002, 300003, 300004, 300005,
];

const BASE_PRICES: &[i64] = &[
    5000000, 10000000, 15000000, 25000000, 50000000,
    100000, 500000, 1000000, 3000000, 20000000,
];

const SYMBOLS: &[&str] = &[
    "BTCUSD", "ETHUSD", "SOLUSD", "ADAUSD", "DOTUSD",
    "AAPL", "MSFT", "GOOGL", "AMZN", "TSLA", "NVDA",
    "JPM", "BAC", "GS", "MS", "C",
    "EURUSD", "GBPUSD", "USDJPY", "AUDUSD",
];

const BASE_TIMESTAMP: u64 = 1704067200000000000;

pub const MIXED_TICK_RATIO: f64 = 0.70;
pub const MIXED_ORDER_RATIO: f64 = 0.20;
pub const MIXED_BOOK_SMALL_RATIO: f64 = 0.07;
pub const MIXED_BOOK_MEDIUM_RATIO: f64 = 0.02;

const BUY_PROBABILITY: f64 = 0.52;
const LIMIT_ORDER_PROBABILITY: f64 = 0.85;

pub enum Message {
    Tick(Tick),
    Order(Order),
    OrderBook(OrderBook),
}

pub enum Scenario {
    TickStreaming,
    OrderEntry,
    OrderBookSmall,
    OrderBookMedium,
    OrderBookLarge,
    MixedWorkload,
    BurstTraffic,
}

impl Scenario {
    pub fn name(&self) -> &'static str {
        match self {
            Scenario::TickStreaming => "Tick Streaming",
            Scenario::OrderEntry => "Order Entry",
            Scenario::OrderBookSmall => "OrderBook Small (5 levels)",
            Scenario::OrderBookMedium => "OrderBook Medium (20 levels)",
            Scenario::OrderBookLarge => "OrderBook Large (100 levels)",
            Scenario::MixedWorkload => "Mixed Workload",
            Scenario::BurstTraffic => "Burst Traffic",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Scenario::TickStreaming => "High-frequency market data stream",
            Scenario::OrderEntry => "Order submission and acknowledgment",
            Scenario::OrderBookSmall => "Typical order book snapshot",
            Scenario::OrderBookMedium => "Liquid instrument order book",
            Scenario::OrderBookLarge => "Deep market order book",
            Scenario::MixedWorkload => "Realistic trading session (70% ticks, 20% orders, 7% small books, 2% medium books, 1% large books)",
            Scenario::BurstTraffic => "Peak load during market events",
        }
    }

    pub fn sample_count(&self) -> usize {
        match self {
            Scenario::TickStreaming => 1_000_000,
            Scenario::OrderEntry => 500_000,
            Scenario::OrderBookSmall => 100_000,
            Scenario::OrderBookMedium => 100_000,
            Scenario::OrderBookLarge => 100_000,
            Scenario::MixedWorkload => 1_000_000,
            Scenario::BurstTraffic => 1_000_000,
        }
    }

    pub fn generate_messages(&self) -> Vec<Message> {
        let seed = 42;
        match self {
            Scenario::TickStreaming => generate_ticks_high_entropy(self.sample_count(), seed),
            Scenario::OrderEntry => generate_orders_high_entropy(self.sample_count(), seed),
            Scenario::OrderBookSmall => generate_order_books_high_entropy(self.sample_count(), 5, seed),
            Scenario::OrderBookMedium => generate_order_books_high_entropy(self.sample_count(), 20, seed),
            Scenario::OrderBookLarge => generate_order_books_high_entropy(self.sample_count(), 100, seed),
            Scenario::MixedWorkload => generate_mixed_workload(self.sample_count(), seed),
            Scenario::BurstTraffic => generate_burst_traffic(self.sample_count(), seed),
        }
    }
}

fn generate_tick(rng: &mut StdRng, seq_num: u64, base_ts: u64) -> Tick {
    let instrument_idx = rng.random_range(0..INSTRUMENTS.len());
    let price_idx = rng.random_range(0..BASE_PRICES.len());
    let price_variance = rng.random_range(-5000..5000);
    let quantity_base = rng.random_range(1000..100000);
    let side = if rng.random_bool(BUY_PROBABILITY) { Side::Buy } else { Side::Sell };
    let ts_jitter = rng.random_range(0..10000);

    Tick {
        instrument_id: INSTRUMENTS[instrument_idx],
        exchange_ts_ns: base_ts + (seq_num * 1000) + ts_jitter,
        ingest_ts_ns: base_ts + (seq_num * 1000) + ts_jitter + rng.random_range(100..1000),
        seq_num,
        price: BASE_PRICES[price_idx] + price_variance,
        quantity: quantity_base + rng.random_range(0..10000),
        side,
        trade_id: rng.random_range(1000000..9999999),
    }
}

fn generate_order(rng: &mut StdRng, order_id: u64, base_ts: u64) -> Order {
    let symbol_idx = rng.random_range(0..SYMBOLS.len());
    let price_idx = rng.random_range(0..BASE_PRICES.len());
    let price_variance = rng.random_range(-10000..10000);
    let quantity_base = rng.random_range(1000..50000);
    let side = if rng.random_bool(1.0 - BUY_PROBABILITY) { Side::Buy } else { Side::Sell };
    let order_type = if rng.random_bool(LIMIT_ORDER_PROBABILITY) { OrderType::Limit } else { OrderType::Market };
    let ts_jitter = rng.random_range(0..20000);
    let instrument_id = 100000 + rng.random_range(1..100);

    Order {
        instrument_id,
        symbol: SYMBOLS[symbol_idx].to_string(),
        order_id,
        client_order_id: format!("CL{:010}_{}", rng.random_range(1000000..9999999), order_id % 1000),
        client_ts_ns: base_ts + (order_id * 2000) + ts_jitter,
        side,
        order_type,
        price: BASE_PRICES[price_idx] + price_variance,
        quantity: quantity_base + rng.random_range(0..20000),
    }
}

fn generate_order_book(rng: &mut StdRng, seq_num: u64, levels: usize, base_ts: u64) -> OrderBook {
    let instrument_idx = rng.random_range(0..INSTRUMENTS.len());
    let price_idx = rng.random_range(0..BASE_PRICES.len());
    let base_price = BASE_PRICES[price_idx];
    let ts_jitter = rng.random_range(0..50000);
    
    let actual_levels = if levels > 20 {
        rng.random_range((levels * 7 / 10)..=levels)
    } else {
        levels
    };

    let mut bids = Vec::with_capacity(actual_levels);
    let mut asks = Vec::with_capacity(actual_levels);

    for level in 0..actual_levels {
        let spread = (level as i64 + 1) * rng.random_range(50..200);
        let quantity_variance = rng.random_range(0..5000);
        
        bids.push(PriceLevel {
            price: base_price - spread,
            quantity: 10000 + (level as i64 * 1000) + quantity_variance,
        });

        asks.push(PriceLevel {
            price: base_price + spread,
            quantity: 10000 + (level as i64 * 1000) + quantity_variance,
        });
    }

    OrderBook {
        instrument_id: INSTRUMENTS[instrument_idx],
        exchange_ts_ns: base_ts + (seq_num * 10000) + ts_jitter,
        ingest_ts_ns: base_ts + (seq_num * 10000) + ts_jitter + rng.random_range(500..2000),
        seq_num,
        bids,
        asks,
    }
}

fn generate_ticks_high_entropy(count: usize, seed: u64) -> Vec<Message> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        messages.push(Message::Tick(generate_tick(&mut rng, i as u64, BASE_TIMESTAMP)));
    }

    messages
}

fn generate_orders_high_entropy(count: usize, seed: u64) -> Vec<Message> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        messages.push(Message::Order(generate_order(&mut rng, 1000000 + i as u64, BASE_TIMESTAMP)));
    }

    messages
}

fn generate_order_books_high_entropy(count: usize, levels: usize, seed: u64) -> Vec<Message> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        messages.push(Message::OrderBook(generate_order_book(&mut rng, i as u64, levels, BASE_TIMESTAMP)));
    }

    messages
}

fn generate_mixed_workload(count: usize, seed: u64) -> Vec<Message> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        let selector: f64 = rng.random();
        
        if selector < MIXED_TICK_RATIO {
            messages.push(Message::Tick(generate_tick(&mut rng, i as u64, BASE_TIMESTAMP)));
        } else if selector < MIXED_TICK_RATIO + MIXED_ORDER_RATIO {
            messages.push(Message::Order(generate_order(&mut rng, 1000000 + i as u64, BASE_TIMESTAMP)));
        } else if selector < MIXED_TICK_RATIO + MIXED_ORDER_RATIO + MIXED_BOOK_SMALL_RATIO {
            messages.push(Message::OrderBook(generate_order_book(&mut rng, i as u64, 5, BASE_TIMESTAMP)));
        } else if selector < MIXED_TICK_RATIO + MIXED_ORDER_RATIO + MIXED_BOOK_SMALL_RATIO + MIXED_BOOK_MEDIUM_RATIO {
            messages.push(Message::OrderBook(generate_order_book(&mut rng, i as u64, 20, BASE_TIMESTAMP)));
        } else {
            messages.push(Message::OrderBook(generate_order_book(&mut rng, i as u64, 100, BASE_TIMESTAMP)));
        }
    }

    messages
}

fn generate_burst_traffic(count: usize, seed: u64) -> Vec<Message> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);
    
    let normal_size = count * 9 / 10;
    let burst_size = count - normal_size;

    for i in 0..normal_size {
        let selector: f64 = rng.random();
        
        if selector < MIXED_TICK_RATIO {
            messages.push(Message::Tick(generate_tick(&mut rng, i as u64, BASE_TIMESTAMP)));
        } else if selector < MIXED_TICK_RATIO + MIXED_ORDER_RATIO {
            messages.push(Message::Order(generate_order(&mut rng, 1000000 + i as u64, BASE_TIMESTAMP)));
        } else {
            messages.push(Message::OrderBook(generate_order_book(&mut rng, i as u64, 5, BASE_TIMESTAMP)));
        }
    }

    for i in 0..burst_size {
        messages.push(Message::Tick(generate_tick(&mut rng, (normal_size + i) as u64, BASE_TIMESTAMP)));
    }

    messages
}

pub fn all_scenarios() -> Vec<Scenario> {
    vec![
        Scenario::TickStreaming,
        Scenario::OrderEntry,
        Scenario::OrderBookSmall,
        Scenario::OrderBookMedium,
        Scenario::OrderBookLarge,
        Scenario::MixedWorkload,
        Scenario::BurstTraffic,
    ]
}
