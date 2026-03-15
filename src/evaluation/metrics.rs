//! Metrics, recorders, and run-level result types (Section IV-A).
//!
//! Paper metrics computed from these structures:
//! - TLP (Tail Latency Profile) = `p99_ns` of round-trip (Section IV-A.2)
//! - TAR (Tail Amplification Ratio) = `tail_amplification_p99` = p99/p50 (Section IV-A.3)
//! - LSC (Latency Stability Coefficient) = `lsc` = MAD/median (Section IV-A.4)
//! - SE  (Size Efficiency) = `median_bytes` (Section IV-A.5)
//! - TP  (Throughput) = `throughput_msg_per_sec` over 5s window (Section IV-A.6)

use hdrhistogram::Histogram;
use serde::Serialize;
use std::time::Duration;

/// Per-histogram latency distribution summary.
/// All latency values are in nanoseconds (integer precision).
#[derive(Debug, Clone, Serialize)]
pub struct LatencyStats {
    pub mean_ns: f64,
    pub median_ns: u64,       // p50 — used as TAR denominator
    pub p90_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,          // TLP primary metric (Section IV-A.2)
    pub p999_ns: u64,         // reported but not in composite score
    pub p9999_ns: u64,        // reported but not in composite score
    pub min_ns: u64,
    pub max_ns: u64,
    pub std_dev_ns: f64,
    pub jitter_coefficient: f64, // CV = σ/μ — reported for prior-work comparability (Section IV-A.4)
    pub lsc: f64,                // LSC = MAD/median — primary stability metric (Section IV-A.4)
    pub sample_count: u64,
    pub tail_amplification_p99: f64,   // TAR = p99/p50 (Section IV-A.3)
    pub tail_amplification_p999: f64,  // diagnostic: p99.9/p50
    pub tail_amplification_p9999: f64, // diagnostic: p99.99/p50
}

#[derive(Debug, Clone, Serialize)]
pub struct SizeStats {
    pub median_bytes: f64,
    pub mean_bytes: f64,
    pub min_bytes: usize,
    pub max_bytes: usize,
    pub total_bytes: usize,
    pub sample_count: usize,
}

#[derive(Debug, Clone)]
pub struct ProtocolMetrics {
    pub protocol_name: String,
    pub scenario_name: String,
    pub encode_latency: LatencyStats,
    pub decode_latency: LatencyStats,
    pub message_size: SizeStats,
    pub throughput_msg_per_sec: f64,
    pub throughput_bytes_per_sec: f64,
    pub encode_latency_amplification: Option<f64>,
    pub decode_latency_amplification: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub protocol_name: String,
    pub scenario_name: String,
    pub seed: u64,
    pub run_index: usize,
    pub encode_latency: LatencyStats,
    pub decode_latency: LatencyStats,
    pub roundtrip_latency: LatencyStats,
    pub message_size: SizeStats,
    pub throughput_msg_per_sec: f64,
    pub throughput_bytes_per_sec: f64,
    pub total_messages: usize,
    pub warmup_messages: usize,
    pub measured_messages: usize,
}

/// Records latency samples into an HDR histogram (Section IV-D.3).
///
/// HDR Histogram configured with 3 significant digits of precision,
/// supporting range from 1 ns to ~3.6 seconds (Section IV-D.3).
///
/// `new_with_raw()` additionally stores raw nanosecond values in a Vec
/// for exact MAD/median (LSC) computation. Used only for the round-trip
/// recorder — encode/decode recorders use HDR-only (`new()`).
pub struct LatencyRecorder {
    histogram: Histogram<u64>,
    raw_values: Option<Vec<u64>>,
}

impl LatencyRecorder {
    pub fn new() -> Self {
        Self {
            // 3 significant digits per Section IV-D.3
            histogram: Histogram::<u64>::new(3).expect("Failed to create histogram"),
            raw_values: None,
        }
    }

    /// Create a recorder that also stores raw values for exact LSC computation.
    /// Memory: ~8 bytes per message (1M messages ≈ 8 MB).
    pub fn new_with_raw() -> Self {
        Self {
            histogram: Histogram::<u64>::new(3).expect("Failed to create histogram"),
            raw_values: Some(Vec::new()),
        }
    }

    pub fn record(&mut self, duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        self.histogram.record(nanos).expect("Failed to record latency");
        if let Some(ref mut raw) = self.raw_values {
            raw.push(nanos);
        }
    }

    pub fn record_nanos(&mut self, nanos: u64) {
        self.histogram.record(nanos).expect("Failed to record latency");
        if let Some(ref mut raw) = self.raw_values {
            raw.push(nanos);
        }
    }

    pub fn finalize(&self) -> LatencyStats {
        let mean = self.histogram.mean();
        let std_dev = self.histogram.stdev();
        let median_ns = self.histogram.value_at_quantile(0.50);
        let p99_ns = self.histogram.value_at_quantile(0.99);
        let p999_ns = self.histogram.value_at_quantile(0.999);
        let p9999_ns = self.histogram.value_at_quantile(0.9999);

        let median_f64 = median_ns as f64;
        let tail_amplification_p99 = if median_ns > 0 { p99_ns as f64 / median_f64 } else { 1.0 };
        let tail_amplification_p999 = if median_ns > 0 { p999_ns as f64 / median_f64 } else { 1.0 };
        let tail_amplification_p9999 = if median_ns > 0 { p9999_ns as f64 / median_f64 } else { 1.0 };

        let jitter_coefficient = if mean > 0.0 { std_dev / mean } else { 0.0 };

        // LSC = MAD / median (Section IV-A.4). Computed from raw values when
        // available (round-trip recorder). Falls back to CV (σ/μ) for encode/decode
        // recorders where raw values are not stored.
        let lsc = if let Some(ref raw) = self.raw_values {
            compute_lsc(raw, median_ns)
        } else {
            jitter_coefficient
        };

        LatencyStats {
            mean_ns: mean,
            median_ns,
            p90_ns: self.histogram.value_at_quantile(0.90),
            p95_ns: self.histogram.value_at_quantile(0.95),
            p99_ns,
            p999_ns,
            p9999_ns,
            min_ns: self.histogram.min(),
            max_ns: self.histogram.max(),
            std_dev_ns: std_dev,
            jitter_coefficient,
            lsc,
            sample_count: self.histogram.len(),
            tail_amplification_p99,
            tail_amplification_p999,
            tail_amplification_p9999,
        }
    }

    pub fn reset(&mut self) {
        self.histogram.reset();
        if let Some(ref mut raw) = self.raw_values {
            raw.clear();
        }
    }
}

/// Compute Latency Stability Coefficient (Section IV-A.4):
///   LSC = MAD / median,  where MAD = median(|x_i − median(x)|).
/// MAD is robust to heavy tails (unlike σ in CV), avoiding circular
/// dependency on the tail behavior being measured.
fn compute_lsc(raw: &[u64], median: u64) -> f64 {
    if raw.is_empty() || median == 0 {
        return 0.0;
    }

    let mut deviations: Vec<u64> = raw
        .iter()
        .map(|&x| if x > median { x - median } else { median - x })
        .collect();
    deviations.sort_unstable();

    let mad = if deviations.len() % 2 == 0 {
        let mid = deviations.len() / 2;
        (deviations[mid - 1] + deviations[mid]) / 2
    } else {
        deviations[deviations.len() / 2]
    };

    mad as f64 / median as f64
}

impl Default for LatencyRecorder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SizeRecorder {
    sizes: Vec<usize>,
    total_bytes: usize,
}

impl SizeRecorder {
    pub fn new() -> Self {
        Self {
            sizes: Vec::new(),
            total_bytes: 0,
        }
    }

    pub fn record(&mut self, size: usize) {
        self.sizes.push(size);
        self.total_bytes += size;
    }

    pub fn finalize(&self) -> SizeStats {
        if self.sizes.is_empty() {
            return SizeStats {
                median_bytes: 0.0,
                mean_bytes: 0.0,
                min_bytes: 0,
                max_bytes: 0,
                total_bytes: 0,
                sample_count: 0,
            };
        }

        let mut sorted_sizes = self.sizes.clone();
        sorted_sizes.sort_unstable();

        let median_bytes = if sorted_sizes.len() % 2 == 0 {
            let mid = sorted_sizes.len() / 2;
            (sorted_sizes[mid - 1] + sorted_sizes[mid]) as f64 / 2.0
        } else {
            sorted_sizes[sorted_sizes.len() / 2] as f64
        };

        let min_bytes = sorted_sizes[0];
        let max_bytes = sorted_sizes[sorted_sizes.len() - 1];
        let mean_bytes = self.total_bytes as f64 / self.sizes.len() as f64;

        SizeStats {
            median_bytes,
            mean_bytes,
            min_bytes,
            max_bytes,
            total_bytes: self.total_bytes,
            sample_count: self.sizes.len(),
        }
    }

    pub fn reset(&mut self) {
        self.sizes.clear();
        self.total_bytes = 0;
    }
}

impl Default for SizeRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyStats {
    pub fn to_micros(&self) -> LatencyStatsMicros {
        LatencyStatsMicros {
            mean: self.mean_ns / 1000.0,
            median: self.median_ns as f64 / 1000.0,
            p90: self.p90_ns as f64 / 1000.0,
            p95: self.p95_ns as f64 / 1000.0,
            p99: self.p99_ns as f64 / 1000.0,
            p999: self.p999_ns as f64 / 1000.0,
            p9999: self.p9999_ns as f64 / 1000.0,
            min: self.min_ns as f64 / 1000.0,
            max: self.max_ns as f64 / 1000.0,
            std_dev: self.std_dev_ns / 1000.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LatencyStatsMicros {
    pub mean: f64,
    pub median: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
    pub p9999: f64,
    pub min: f64,
    pub max: f64,
    pub std_dev: f64,
}

impl ProtocolMetrics {
    pub fn print_summary(&self) {
        println!("\n=== {} - {} ===", self.protocol_name, self.scenario_name);

        let encode_us = self.encode_latency.to_micros();
        let decode_us = self.decode_latency.to_micros();

        println!("\nEncode Latency (us):");
        println!("  P50:    {:.2}", encode_us.median);
        println!("  P99:    {:.2} ({:.2}x)", encode_us.p99, self.encode_latency.tail_amplification_p99);
        println!("  P99.9:  {:.2} ({:.2}x)", encode_us.p999, self.encode_latency.tail_amplification_p999);
        println!("  P99.99: {:.2} ({:.2}x)", encode_us.p9999, self.encode_latency.tail_amplification_p9999);
        println!("  Jitter: {:.3}", self.encode_latency.jitter_coefficient);
        if let Some(amp) = self.encode_latency_amplification {
            println!("  Amplification: {:.2}x", amp);
        }

        println!("\nDecode Latency (us):");
        println!("  P50:    {:.2}", decode_us.median);
        println!("  P99:    {:.2} ({:.2}x)", decode_us.p99, self.decode_latency.tail_amplification_p99);
        println!("  P99.9:  {:.2} ({:.2}x)", decode_us.p999, self.decode_latency.tail_amplification_p999);
        println!("  P99.99: {:.2} ({:.2}x)", decode_us.p9999, self.decode_latency.tail_amplification_p9999);
        println!("  Jitter: {:.3}", self.decode_latency.jitter_coefficient);
        if let Some(amp) = self.decode_latency_amplification {
            println!("  Amplification: {:.2}x", amp);
        }

        println!("\nMessage Size:");
        println!("  Median: {:.0} bytes", self.message_size.median_bytes);
        println!("  Mean:   {:.1} bytes", self.message_size.mean_bytes);
        println!("  Range:  {} - {} bytes", self.message_size.min_bytes, self.message_size.max_bytes);

        println!("\nThroughput (steady state):");
        println!("  {:.0} msg/sec", self.throughput_msg_per_sec);
        println!("  {:.2} MB/sec", self.throughput_bytes_per_sec / 1_000_000.0);
        println!("  {:.2} bytes/msg", self.message_size.median_bytes);
    }

    pub fn print_csv_header() {
        println!("protocol,scenario,encode_p50,encode_p99,encode_p999,encode_p9999,encode_jitter,encode_tail_p99,encode_tail_p999,encode_tail_p9999,encode_amp,decode_p50,decode_p99,decode_p999,decode_p9999,decode_jitter,decode_tail_p99,decode_tail_p999,decode_tail_p9999,decode_amp,size_median,size_mean,size_min,size_max,throughput_msg_sec,throughput_mb_sec,bytes_per_msg");
    }

    pub fn print_csv_row(&self) {
        let encode_us = self.encode_latency.to_micros();
        let decode_us = self.decode_latency.to_micros();

        let encode_amp = self.encode_latency_amplification.map(|a| format!("{:.2}", a)).unwrap_or_else(|| "1.00".to_string());
        let decode_amp = self.decode_latency_amplification.map(|a| format!("{:.2}", a)).unwrap_or_else(|| "1.00".to_string());

        println!(
            "{},{},{:.2},{:.2},{:.2},{:.2},{:.3},{:.2},{:.2},{:.2},{},{:.2},{:.2},{:.2},{:.2},{:.3},{:.2},{:.2},{:.2},{},{:.0},{:.1},{},{},{:.0},{:.2},{:.0}",
            self.protocol_name,
            self.scenario_name,
            encode_us.median,
            encode_us.p99,
            encode_us.p999,
            encode_us.p9999,
            self.encode_latency.jitter_coefficient,
            self.encode_latency.tail_amplification_p99,
            self.encode_latency.tail_amplification_p999,
            self.encode_latency.tail_amplification_p9999,
            encode_amp,
            decode_us.median,
            decode_us.p99,
            decode_us.p999,
            decode_us.p9999,
            self.decode_latency.jitter_coefficient,
            self.decode_latency.tail_amplification_p99,
            self.decode_latency.tail_amplification_p999,
            self.decode_latency.tail_amplification_p9999,
            decode_amp,
            self.message_size.median_bytes,
            self.message_size.mean_bytes,
            self.message_size.min_bytes,
            self.message_size.max_bytes,
            self.throughput_msg_per_sec,
            self.throughput_bytes_per_sec / 1_000_000.0,
            self.message_size.median_bytes,
        );
    }
}

impl RunResult {
    pub fn csv_header() -> String {
        "protocol,scenario,seed,run_index,\
         encode_p50_ns,encode_p99_ns,encode_p999_ns,encode_p9999_ns,encode_mean_ns,encode_min_ns,encode_max_ns,encode_stddev_ns,encode_cv,encode_tar_p99,encode_tar_p999,encode_tar_p9999,\
         decode_p50_ns,decode_p99_ns,decode_p999_ns,decode_p9999_ns,decode_mean_ns,decode_min_ns,decode_max_ns,decode_stddev_ns,decode_cv,decode_tar_p99,decode_tar_p999,decode_tar_p9999,\
         rt_p50_ns,rt_p99_ns,rt_p999_ns,rt_p9999_ns,rt_mean_ns,rt_min_ns,rt_max_ns,rt_stddev_ns,rt_cv,rt_lsc,rt_tar_p99,rt_tar_p999,rt_tar_p9999,\
         size_median,size_mean,size_min,size_max,\
         throughput_msg_sec,throughput_bytes_sec,\
         total_messages,warmup_messages,measured_messages"
            .to_string()
    }

    pub fn to_csv_row(&self) -> String {
        fn lat(s: &LatencyStats) -> String {
            format!(
                "{},{},{},{},{:.1},{},{},{:.1},{:.6},{:.4},{:.4},{:.4}",
                s.median_ns, s.p99_ns, s.p999_ns, s.p9999_ns,
                s.mean_ns, s.min_ns, s.max_ns, s.std_dev_ns,
                s.jitter_coefficient,
                s.tail_amplification_p99, s.tail_amplification_p999, s.tail_amplification_p9999,
            )
        }

        let rt = &self.roundtrip_latency;
        let rt_csv = format!(
            "{},{},{},{},{:.1},{},{},{:.1},{:.6},{:.6},{:.4},{:.4},{:.4}",
            rt.median_ns, rt.p99_ns, rt.p999_ns, rt.p9999_ns,
            rt.mean_ns, rt.min_ns, rt.max_ns, rt.std_dev_ns,
            rt.jitter_coefficient, rt.lsc,
            rt.tail_amplification_p99, rt.tail_amplification_p999, rt.tail_amplification_p9999,
        );

        format!(
            "{},{},{},{},{},{},{},{:.1},{:.1},{},{},{:.1},{:.1},{},{},{}",
            self.protocol_name,
            self.scenario_name,
            self.seed,
            self.run_index,
            lat(&self.encode_latency),
            lat(&self.decode_latency),
            rt_csv,
            self.message_size.median_bytes,
            self.message_size.mean_bytes,
            self.message_size.min_bytes,
            self.message_size.max_bytes,
            self.throughput_msg_per_sec,
            self.throughput_bytes_per_sec,
            self.total_messages,
            self.warmup_messages,
            self.measured_messages,
        )
    }
}
