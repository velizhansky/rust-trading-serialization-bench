//! Benchmark runner implementing the three-phase measurement procedure
//! from Section IV-D:
//!
//!   Phase 1 — Warmup (5,000 messages, excluded from measurement)
//!   Phase 2 — Throughput (5-second continuous window, immediately after warmup)
//!   Phase 3 — Latency (per-message encode/decode/round-trip timing)
//!
//! Each invocation of `evaluate_single_run` measures one (protocol, scenario, seed)
//! combination. The shell orchestrator (scripts/run_experiment.sh) restarts the
//! process between runs for clean allocator state (Section IV-C.1).

use crate::evaluation::metrics::{LatencyRecorder, SizeRecorder, ProtocolMetrics, RunResult};
use crate::evaluation::scenarios::{Scenario, Message};
use crate::messages::{Tick, Order, OrderBook};
use crate::protocols;
use std::time::{Duration, Instant};
use std::hint::black_box;

/// Warmup iterations excluded from measurement (Section IV-C.2).
/// Pilot experiments confirmed stabilization within first 5,000 iterations.
const WARMUP_MESSAGES: usize = 5_000;

/// Throughput measurement window in seconds (Section IV-A.6).
/// Guarantees ≥500K operations even for slowest protocol configurations.
const THROUGHPUT_DURATION_SECS: u64 = 5;

pub struct EvaluationConfig {
    pub protocols: Vec<ProtocolType>,
    pub scenarios: Vec<Scenario>,
    pub baseline_protocol: ProtocolType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    Json,
    Bincode,
    Rkyv,
    Protobuf,
    FlatBuffers,
}

impl ProtocolType {
    pub fn name(&self) -> &'static str {
        match self {
            ProtocolType::Json => "JSON",
            ProtocolType::Bincode => "Bincode",
            ProtocolType::Rkyv => "Rkyv",
            ProtocolType::Protobuf => "Protobuf",
            ProtocolType::FlatBuffers => "FlatBuffers",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            ProtocolType::Json => "json",
            ProtocolType::Bincode => "bincode",
            ProtocolType::Rkyv => "rkyv",
            ProtocolType::Protobuf => "protobuf",
            ProtocolType::FlatBuffers => "flatbuffers",
        }
    }

    pub fn from_short_name(name: &str) -> Option<Self> {
        match name {
            "json" => Some(ProtocolType::Json),
            "bincode" => Some(ProtocolType::Bincode),
            "rkyv" => Some(ProtocolType::Rkyv),
            "protobuf" => Some(ProtocolType::Protobuf),
            "flatbuffers" => Some(ProtocolType::FlatBuffers),
            _ => None,
        }
    }

    pub fn all() -> Vec<ProtocolType> {
        vec![
            ProtocolType::Json,
            ProtocolType::Bincode,
            ProtocolType::Rkyv,
            ProtocolType::Protobuf,
            ProtocolType::FlatBuffers,
        ]
    }
}

pub struct EvaluationRunner {
    config: EvaluationConfig,
}

impl EvaluationRunner {
    pub fn new(config: EvaluationConfig) -> Self {
        Self { config }
    }

    /// Legacy method: runs all (protocol, scenario) pairs with seed=42.
    /// Kept for backward compatibility with existing tests.
    pub fn run(&self) -> Vec<ProtocolMetrics> {
        let mut all_metrics = Vec::new();
        let mut baseline_metrics = std::collections::HashMap::new();

        for scenario in &self.config.scenarios {
            println!("\n{}", "=".repeat(60));
            println!("Scenario: {}", scenario.name());
            println!("{}", "=".repeat(60));

            for protocol in &self.config.protocols {
                println!("\nEvaluating {} with {}...", protocol.name(), scenario.name());

                let run_result = self.evaluate_single_run(*protocol, scenario, 42, 0);

                let metrics = ProtocolMetrics {
                    protocol_name: protocol.name().to_string(),
                    scenario_name: scenario.name().to_string(),
                    encode_latency: run_result.encode_latency,
                    decode_latency: run_result.decode_latency,
                    message_size: run_result.message_size,
                    throughput_msg_per_sec: run_result.throughput_msg_per_sec,
                    throughput_bytes_per_sec: run_result.throughput_bytes_per_sec,
                    encode_latency_amplification: None,
                    decode_latency_amplification: None,
                };

                if *protocol == self.config.baseline_protocol {
                    baseline_metrics.insert(scenario.name().to_string(), metrics.clone());
                }

                all_metrics.push(metrics);
            }
        }

        self.compute_amplifications(&mut all_metrics, &baseline_metrics);
        all_metrics
    }

    /// Evaluate a single (protocol, scenario, seed) combination.
    ///
    /// Measurement order (Section IV-D, Figure 2):
    ///   1. Warmup — first `WARMUP_MESSAGES` messages, encode+decode, results discarded
    ///   2. Throughput — immediately after warmup (Section IV-A.6), 5s continuous window
    ///   3. Latency — remaining messages, per-message timing into 3 HDR histograms
    ///
    /// The throughput phase runs BEFORE latency to match the paper specification:
    /// "measured over the first continuous T-second interval following warmup completion."
    pub fn evaluate_single_run(
        &self,
        protocol: ProtocolType,
        scenario: &Scenario,
        seed: u64,
        run_index: usize,
    ) -> RunResult {
        let messages = scenario.generate_messages_with_seed(seed);
        assert!(
            WARMUP_MESSAGES < messages.len(),
            "Scenario {} has {} messages, need at least {} for warmup",
            scenario.name(),
            messages.len(),
            WARMUP_MESSAGES + 1
        );

        // Phase 1: Warmup — first WARMUP_MESSAGES messages
        self.warmup(protocol, &messages[..WARMUP_MESSAGES]);

        // Phase 2: Throughput — immediately after warmup (per Section IV-A.3)
        let (throughput_msg_per_sec, throughput_bytes_per_sec) =
            self.measure_throughput(protocol, &messages);

        // Phase 3: Latency measurement — messages after warmup.
        // Three separate HDR histograms per Section IV-D.3.
        // Round-trip recorder stores raw values for exact LSC (MAD/median) computation.
        // Every message is measured individually (no subsampling — Section IV-D.4).
        let mut encode_latency = LatencyRecorder::new();
        let mut decode_latency = LatencyRecorder::new();
        let mut roundtrip_latency = LatencyRecorder::new_with_raw();
        let mut size_recorder = SizeRecorder::new();

        for (i, message) in messages[WARMUP_MESSAGES..].iter().enumerate() {
            let (encoded, encode_time, decode_time) = match message {
                Message::Tick(tick) => self.bench_tick(protocol, tick, i),
                Message::Order(order) => self.bench_order(protocol, order, i),
                Message::OrderBook(book) => self.bench_order_book(protocol, book, i),
            };

            encode_latency.record(encode_time);
            decode_latency.record(decode_time);
            roundtrip_latency.record_nanos(
                encode_time.as_nanos() as u64 + decode_time.as_nanos() as u64,
            );
            size_recorder.record(encoded.len());
        }

        let measured_messages = messages.len() - WARMUP_MESSAGES;

        RunResult {
            protocol_name: protocol.short_name().to_string(),
            scenario_name: scenario.short_name().to_string(),
            seed,
            run_index,
            encode_latency: encode_latency.finalize(),
            decode_latency: decode_latency.finalize(),
            roundtrip_latency: roundtrip_latency.finalize(),
            message_size: size_recorder.finalize(),
            throughput_msg_per_sec,
            throughput_bytes_per_sec,
            total_messages: messages.len(),
            warmup_messages: WARMUP_MESSAGES,
            measured_messages,
        }
    }

    fn warmup(&self, protocol: ProtocolType, messages: &[Message]) {
        for (i, message) in messages.iter().enumerate() {
            match message {
                Message::Tick(tick) => {
                    let encoded = self.encode_tick(protocol, tick);
                    let decoded = self.decode_tick(protocol, &encoded);
                    if i % 100 == 0 {
                        assert_eq!(tick.instrument_id, decoded.instrument_id);
                    }
                    black_box((encoded, decoded));
                }
                Message::Order(order) => {
                    let encoded = self.encode_order(protocol, order);
                    let decoded = self.decode_order(protocol, &encoded);
                    if i % 100 == 0 {
                        assert_eq!(order.order_id, decoded.order_id);
                    }
                    black_box((encoded, decoded));
                }
                Message::OrderBook(book) => {
                    let encoded = self.encode_order_book(protocol, book);
                    let decoded = self.decode_order_book(protocol, &encoded);
                    if i % 100 == 0 {
                        assert_eq!(book.instrument_id, decoded.instrument_id);
                    }
                    black_box((encoded, decoded));
                }
            }
        }
    }

    fn measure_throughput(&self, protocol: ProtocolType, messages: &[Message]) -> (f64, f64) {
        if messages.is_empty() {
            return (0.0, 0.0);
        }

        let deadline = Duration::from_secs(THROUGHPUT_DURATION_SECS);
        let start = Instant::now();
        let mut total_bytes = 0usize;
        let mut total_messages = 0usize;

        while start.elapsed() < deadline {
            for message in messages {
                let size = match message {
                    Message::Tick(tick) => {
                        let enc = self.encode_tick(protocol, tick);
                        let size = enc.len();
                        let dec = self.decode_tick(protocol, &enc);
                        black_box((enc, dec));
                        size
                    }
                    Message::Order(order) => {
                        let enc = self.encode_order(protocol, order);
                        let size = enc.len();
                        let dec = self.decode_order(protocol, &enc);
                        black_box((enc, dec));
                        size
                    }
                    Message::OrderBook(book) => {
                        let enc = self.encode_order_book(protocol, book);
                        let size = enc.len();
                        let dec = self.decode_order_book(protocol, &enc);
                        black_box((enc, dec));
                        size
                    }
                };
                total_bytes += size;
                total_messages += 1;

                if start.elapsed() >= deadline {
                    break;
                }
            }
        }

        let elapsed_secs = start.elapsed().as_secs_f64();
        let msg_per_sec = total_messages as f64 / elapsed_secs;
        let bytes_per_sec = total_bytes as f64 / elapsed_secs;

        (msg_per_sec, bytes_per_sec)
    }

    fn bench_tick(&self, protocol: ProtocolType, tick: &Tick, iteration: usize) -> (Vec<u8>, std::time::Duration, std::time::Duration) {
        let start = Instant::now();
        let encoded = self.encode_tick(protocol, tick);
        let encode_time = start.elapsed();

        let start = Instant::now();
        let decoded = self.decode_tick(protocol, &encoded);
        let decode_time = start.elapsed();

        if iteration % 1000 == 0 {
            assert_eq!(tick.instrument_id, decoded.instrument_id);
        }
        black_box(&decoded);
        (encoded, encode_time, decode_time)
    }

    fn bench_order(&self, protocol: ProtocolType, order: &Order, iteration: usize) -> (Vec<u8>, std::time::Duration, std::time::Duration) {
        let start = Instant::now();
        let encoded = self.encode_order(protocol, order);
        let encode_time = start.elapsed();

        let start = Instant::now();
        let decoded = self.decode_order(protocol, &encoded);
        let decode_time = start.elapsed();

        if iteration % 1000 == 0 {
            assert_eq!(order.order_id, decoded.order_id);
        }
        black_box(&decoded);
        (encoded, encode_time, decode_time)
    }

    fn bench_order_book(&self, protocol: ProtocolType, book: &OrderBook, iteration: usize) -> (Vec<u8>, std::time::Duration, std::time::Duration) {
        let start = Instant::now();
        let encoded = self.encode_order_book(protocol, book);
        let encode_time = start.elapsed();

        let start = Instant::now();
        let decoded = self.decode_order_book(protocol, &encoded);
        let decode_time = start.elapsed();

        if iteration % 1000 == 0 {
            assert_eq!(book.instrument_id, decoded.instrument_id);
        }
        black_box(&decoded);
        (encoded, encode_time, decode_time)
    }

    fn encode_tick(&self, protocol: ProtocolType, tick: &Tick) -> Vec<u8> {
        match protocol {
            ProtocolType::Json => protocols::json::encode_tick(tick),
            ProtocolType::Bincode => protocols::bincode::encode_tick(tick),
            ProtocolType::Rkyv => protocols::rkyv::encode_tick(tick),
            ProtocolType::Protobuf => protocols::protobuf::encode_tick(tick),
            ProtocolType::FlatBuffers => protocols::flatbuffers::encode_tick(tick),
        }
    }

    fn decode_tick(&self, protocol: ProtocolType, bytes: &[u8]) -> Tick {
        match protocol {
            ProtocolType::Json => protocols::json::decode_tick(bytes),
            ProtocolType::Bincode => protocols::bincode::decode_tick(bytes),
            ProtocolType::Rkyv => protocols::rkyv::decode_tick(bytes),
            ProtocolType::Protobuf => protocols::protobuf::decode_tick(bytes),
            ProtocolType::FlatBuffers => protocols::flatbuffers::decode_tick(bytes),
        }
    }

    fn encode_order(&self, protocol: ProtocolType, order: &Order) -> Vec<u8> {
        match protocol {
            ProtocolType::Json => protocols::json::encode_order(order),
            ProtocolType::Bincode => protocols::bincode::encode_order(order),
            ProtocolType::Rkyv => protocols::rkyv::encode_order(order),
            ProtocolType::Protobuf => protocols::protobuf::encode_order(order),
            ProtocolType::FlatBuffers => protocols::flatbuffers::encode_order(order),
        }
    }

    fn decode_order(&self, protocol: ProtocolType, bytes: &[u8]) -> Order {
        match protocol {
            ProtocolType::Json => protocols::json::decode_order(bytes),
            ProtocolType::Bincode => protocols::bincode::decode_order(bytes),
            ProtocolType::Rkyv => protocols::rkyv::decode_order(bytes),
            ProtocolType::Protobuf => protocols::protobuf::decode_order(bytes),
            ProtocolType::FlatBuffers => protocols::flatbuffers::decode_order(bytes),
        }
    }

    fn encode_order_book(&self, protocol: ProtocolType, book: &OrderBook) -> Vec<u8> {
        match protocol {
            ProtocolType::Json => protocols::json::encode_order_book(book),
            ProtocolType::Bincode => protocols::bincode::encode_order_book(book),
            ProtocolType::Rkyv => protocols::rkyv::encode_order_book(book),
            ProtocolType::Protobuf => protocols::protobuf::encode_order_book(book),
            ProtocolType::FlatBuffers => protocols::flatbuffers::encode_order_book(book),
        }
    }

    fn decode_order_book(&self, protocol: ProtocolType, bytes: &[u8]) -> OrderBook {
        match protocol {
            ProtocolType::Json => protocols::json::decode_order_book(bytes),
            ProtocolType::Bincode => protocols::bincode::decode_order_book(bytes),
            ProtocolType::Rkyv => protocols::rkyv::decode_order_book(bytes),
            ProtocolType::Protobuf => protocols::protobuf::decode_order_book(bytes),
            ProtocolType::FlatBuffers => protocols::flatbuffers::decode_order_book(bytes),
        }
    }

    fn compute_amplifications(
        &self,
        all_metrics: &mut [ProtocolMetrics],
        baseline_metrics: &std::collections::HashMap<String, ProtocolMetrics>,
    ) {
        for metrics in all_metrics.iter_mut() {
            if let Some(baseline) = baseline_metrics.get(&metrics.scenario_name) {
                let encode_amp = metrics.encode_latency.p99_ns as f64
                    / baseline.encode_latency.p99_ns as f64;
                let decode_amp = metrics.decode_latency.p99_ns as f64
                    / baseline.decode_latency.p99_ns as f64;

                metrics.encode_latency_amplification = Some(encode_amp);
                metrics.decode_latency_amplification = Some(decode_amp);
            }
        }
    }
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            protocols: vec![
                ProtocolType::Json,
                ProtocolType::Bincode,
                ProtocolType::Rkyv,
                ProtocolType::Protobuf,
                ProtocolType::FlatBuffers,
            ],
            scenarios: vec![
                Scenario::TickStreaming,
                Scenario::OrderEntry,
                Scenario::OrderBookSmall,
                Scenario::OrderBookMedium,
                Scenario::OrderBookLarge,
                Scenario::MixedWorkload,
                Scenario::BurstTraffic,
            ],
            baseline_protocol: ProtocolType::Json,
        }
    }
}
