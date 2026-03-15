//! Workload generation for evaluation scenarios (Section V-D).
//!
//! Each scenario produces a deterministic sequence of trading messages from a
//! given seed via ChaCha20 PRNG (`StdRng`). Consecutive seeds (42–71) yield
//! 30 distinct but reproducible replications per (protocol, scenario) pair
//! (Section IV-C.1).

use crate::messages::{Tick, Order, OrderBook, PriceLevel, Side, OrderType};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

// --- Generation parameters (Section V-D.4) ---
// 15 numeric instrument IDs spanning 3 asset-class prefixes.
const INSTRUMENTS: &[u64] = &[
    100001, 100002, 100003, 100004, 100005,
    200001, 200002, 200003, 200004, 200005,
    300001, 300002, 300003, 300004, 300005,
];

// 10 base prices in fixed-point units spanning 4 orders of magnitude
// (100K–50M), exercising variable-length integer encoding (e.g. Protobuf varint).
const BASE_PRICES: &[i64] = &[
    5000000, 10000000, 15000000, 25000000, 50000000,
    100000, 500000, 1000000, 3000000, 20000000,
];

// 20 ticker symbols: crypto (5), equities (11), FX (4). Lengths 1–6 chars.
const SYMBOLS: &[&str] = &[
    "BTCUSD", "ETHUSD", "SOLUSD", "ADAUSD", "DOTUSD",
    "AAPL", "MSFT", "GOOGL", "AMZN", "TSLA", "NVDA",
    "JPM", "BAC", "GS", "MS", "C",
    "EURUSD", "GBPUSD", "USDJPY", "AUDUSD",
];

// Jan 1 2024 00:00:00 UTC in nanoseconds — anchor for generated timestamps.
const BASE_TIMESTAMP: u64 = 1704067200000000000;

// S6/S7 mixed workload distribution (Section V-D.2, Table III).
pub const MIXED_TICK_RATIO: f64 = 0.70;
pub const MIXED_ORDER_RATIO: f64 = 0.20;
pub const MIXED_BOOK_SMALL_RATIO: f64 = 0.07;
pub const MIXED_BOOK_MEDIUM_RATIO: f64 = 0.02;
// Remaining 1% = large order books (100 levels).

const BUY_PROBABILITY: f64 = 0.52;       // Section V-D.1: 52% buy / 48% sell
const LIMIT_ORDER_PROBABILITY: f64 = 0.85; // Section V-D.1: 85% limit / 15% market

pub enum Message {
    Tick(Tick),
    Order(Order),
    OrderBook(OrderBook),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn short_name(&self) -> &'static str {
        match self {
            Scenario::TickStreaming => "tick",
            Scenario::OrderEntry => "order",
            Scenario::OrderBookSmall => "book_small",
            Scenario::OrderBookMedium => "book_medium",
            Scenario::OrderBookLarge => "book_large",
            Scenario::MixedWorkload => "mixed",
            Scenario::BurstTraffic => "burst",
        }
    }

    pub fn from_short_name(name: &str) -> Option<Self> {
        match name {
            "tick" => Some(Scenario::TickStreaming),
            "order" => Some(Scenario::OrderEntry),
            "book_small" => Some(Scenario::OrderBookSmall),
            "book_medium" => Some(Scenario::OrderBookMedium),
            "book_large" => Some(Scenario::OrderBookLarge),
            "mixed" => Some(Scenario::MixedWorkload),
            "burst" => Some(Scenario::BurstTraffic),
            _ => None,
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
        self.generate_messages_with_seed(42)
    }

    pub fn generate_messages_with_seed(&self, seed: u64) -> Vec<Message> {
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

/// Generate a single Tick message (Section V-D.1).
/// 8 fields, ~57 bytes logical: fixed-size, simplest serialization case.
fn generate_tick(rng: &mut StdRng, seq_num: u64, base_ts: u64) -> Tick {
    let instrument_idx = rng.random_range(0..INSTRUMENTS.len());
    let price_idx = rng.random_range(0..BASE_PRICES.len());
    let price_variance = rng.random_range(-5000..5000);   // ±5K tick units (Section V-D.1)
    let quantity_base = rng.random_range(1000..100000);    // range 1K–110K
    let side = if rng.random_bool(BUY_PROBABILITY) { Side::Buy } else { Side::Sell };
    let ts_jitter = rng.random_range(0..10000);            // 0–10μs jitter (Section V-F.4)

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

/// Generate a single Order message (Section V-D.2).
/// 9 fields, 60–70 bytes logical: includes 2 variable-length strings
/// (symbol 3–6 chars, client_order_id 15–22 chars).
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

/// Generate a single OrderBook message (Section V-D.3).
/// Nested variable-length arrays of PriceLevel (price: i64, quantity: i64).
/// For S5 (levels=100), actual depth varies 70–100 per side (Section V-D.1)
/// to exercise dynamic buffer sizing.
fn generate_order_book(rng: &mut StdRng, seq_num: u64, levels: usize, base_ts: u64) -> OrderBook {
    let instrument_idx = rng.random_range(0..INSTRUMENTS.len());
    let price_idx = rng.random_range(0..BASE_PRICES.len());
    let base_price = BASE_PRICES[price_idx];
    let ts_jitter = rng.random_range(0..50000); // 0–50μs jitter for book updates

    // S5 (levels>20): random depth between 70% and 100% of target (Section V-D.1).
    // S3/S4 (levels≤20): fixed depth.
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

/// Shared helper: generates a single mixed-distribution message.
/// Used by both MixedWorkload (S6) and the normal phase of BurstTraffic (S7).
/// Distribution: 70% ticks, 20% orders, 7% small books, 2% medium books, 1% large books.
fn generate_mixed_message(rng: &mut StdRng, idx: usize, base_ts: u64) -> Message {
    let selector: f64 = rng.random();

    if selector < MIXED_TICK_RATIO {
        Message::Tick(generate_tick(rng, idx as u64, base_ts))
    } else if selector < MIXED_TICK_RATIO + MIXED_ORDER_RATIO {
        Message::Order(generate_order(rng, 1000000 + idx as u64, base_ts))
    } else if selector < MIXED_TICK_RATIO + MIXED_ORDER_RATIO + MIXED_BOOK_SMALL_RATIO {
        Message::OrderBook(generate_order_book(rng, idx as u64, 5, base_ts))
    } else if selector < MIXED_TICK_RATIO + MIXED_ORDER_RATIO + MIXED_BOOK_SMALL_RATIO + MIXED_BOOK_MEDIUM_RATIO {
        Message::OrderBook(generate_order_book(rng, idx as u64, 20, base_ts))
    } else {
        Message::OrderBook(generate_order_book(rng, idx as u64, 100, base_ts))
    }
}

fn generate_mixed_workload(count: usize, seed: u64) -> Vec<Message> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        messages.push(generate_mixed_message(&mut rng, i, BASE_TIMESTAMP));
    }

    messages
}

fn generate_burst_traffic(count: usize, seed: u64) -> Vec<Message> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    let normal_size = count * 9 / 10;
    let burst_size = count - normal_size;

    // Normal phase: same S6 distribution (70/20/7/2/1)
    for i in 0..normal_size {
        messages.push(generate_mixed_message(&mut rng, i, BASE_TIMESTAMP));
    }

    // Burst phase: 100% ticks (market volatility event)
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
