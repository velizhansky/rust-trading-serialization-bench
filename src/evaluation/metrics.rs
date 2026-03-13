use hdrhistogram::Histogram;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LatencyStats {
    pub mean_ns: f64,
    pub median_ns: u64,
    pub p90_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub p999_ns: u64,
    pub p9999_ns: u64,
    pub min_ns: u64,
    pub max_ns: u64,
    pub std_dev_ns: f64,
    pub jitter_coefficient: f64,
    pub sample_count: u64,
    pub tail_amplification_p99: f64,
    pub tail_amplification_p999: f64,
    pub tail_amplification_p9999: f64,
}

#[derive(Debug, Clone)]
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

pub struct LatencyRecorder {
    histogram: Histogram<u64>,
}

impl LatencyRecorder {
    pub fn new() -> Self {
        Self {
            histogram: Histogram::<u64>::new(3).expect("Failed to create histogram"),
        }
    }

    pub fn record(&mut self, duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        self.histogram.record(nanos).expect("Failed to record latency");
    }

    pub fn record_nanos(&mut self, nanos: u64) {
        self.histogram.record(nanos).expect("Failed to record latency");
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
            sample_count: self.histogram.len(),
            tail_amplification_p99,
            tail_amplification_p999,
            tail_amplification_p9999,
        }
    }

    pub fn reset(&mut self) {
        self.histogram.reset();
    }
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

