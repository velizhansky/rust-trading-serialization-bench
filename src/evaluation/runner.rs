use crate::evaluation::metrics::{LatencyRecorder, SizeRecorder, ProtocolMetrics};
use crate::evaluation::scenarios::{Scenario, Message};
use crate::messages::{Tick, Order, OrderBook};
use crate::protocols;
use std::time::Instant;
use std::hint::black_box;

const WARMUP_ITERATIONS: usize = 1000;
const THROUGHPUT_DURATION_MS: u64 = 1000;

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
}

pub struct EvaluationRunner {
    config: EvaluationConfig,
}

impl EvaluationRunner {
    pub fn new(config: EvaluationConfig) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Vec<ProtocolMetrics> {
        let mut all_metrics = Vec::new();
        let mut baseline_metrics = std::collections::HashMap::new();

        for scenario in &self.config.scenarios {
            println!("\n{}", "=".repeat(60));
            println!("Scenario: {}", scenario.name());
            println!("{}", "=".repeat(60));

            let messages = scenario.generate_messages();

            for protocol in &self.config.protocols {
                println!("\nEvaluating {} with {}...", protocol.name(), scenario.name());
                
                let metrics = self.evaluate_protocol(*protocol, scenario, &messages);
                
                if *protocol == self.config.baseline_protocol {
                    baseline_metrics.insert(scenario.name().to_string(), metrics.clone());
                }
                
                all_metrics.push(metrics);
            }
        }

        self.compute_amplifications(&mut all_metrics, &baseline_metrics);
        all_metrics
    }

    fn evaluate_protocol(
        &self,
        protocol: ProtocolType,
        scenario: &Scenario,
        messages: &[Message],
    ) -> ProtocolMetrics {
        if messages.is_empty() {
            panic!("Cannot evaluate protocol with empty messages");
        }

        let mut encode_latency = LatencyRecorder::new();
        let mut decode_latency = LatencyRecorder::new();
        let mut size_recorder = SizeRecorder::new();

        self.warmup(protocol, messages);

        for (i, message) in messages.iter().enumerate() {
            let (encoded, encode_time, decode_time) = match message {
                Message::Tick(tick) => self.bench_tick(protocol, tick, i),
                Message::Order(order) => self.bench_order(protocol, order, i),
                Message::OrderBook(book) => self.bench_order_book(protocol, book, i),
            };

            encode_latency.record(encode_time);
            decode_latency.record(decode_time);
            size_recorder.record(encoded.len());
        }

        let encode_stats = encode_latency.finalize();
        let decode_stats = decode_latency.finalize();
        let size_stats = size_recorder.finalize();

        let (throughput_msg_per_sec, throughput_bytes_per_sec) = 
            self.measure_throughput(protocol, messages);

        ProtocolMetrics {
            protocol_name: protocol.name().to_string(),
            scenario_name: scenario.name().to_string(),
            encode_latency: encode_stats,
            decode_latency: decode_stats,
            message_size: size_stats,
            throughput_msg_per_sec,
            throughput_bytes_per_sec,
            encode_latency_amplification: None,
            decode_latency_amplification: None,
        }
    }

    fn warmup(&self, protocol: ProtocolType, messages: &[Message]) {
        if messages.is_empty() {
            return;
        }

        for i in 0..WARMUP_ITERATIONS {
            let message = &messages[i % messages.len()];
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

        let start = Instant::now();
        let mut total_bytes = 0usize;
        let mut total_messages = 0usize;
        let duration_ms = THROUGHPUT_DURATION_MS;

        while start.elapsed().as_millis() < duration_ms as u128 {
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

                if start.elapsed().as_millis() >= duration_ms as u128 {
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

