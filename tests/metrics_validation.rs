use rust_trading_serialization_bench::evaluation::metrics::{
    LatencyRecorder, SizeRecorder, RunResult,
};
use std::time::Duration;

#[test]
fn test_latency_recorder() {
    let mut recorder = LatencyRecorder::new();
    
    recorder.record(Duration::from_nanos(100));
    recorder.record(Duration::from_nanos(200));
    recorder.record(Duration::from_nanos(300));
    
    let stats = recorder.finalize();
    
    assert_eq!(stats.sample_count, 3);
    assert_eq!(stats.min_ns, 100);
    assert_eq!(stats.max_ns, 300);
    assert!((stats.mean_ns - 200.0).abs() < 1.0);
}

#[test]
fn test_size_recorder() {
    let mut recorder = SizeRecorder::new();
    
    recorder.record(100);
    recorder.record(200);
    recorder.record(300);
    
    let stats = recorder.finalize();
    
    assert_eq!(stats.sample_count, 3);
    assert_eq!(stats.min_bytes, 100);
    assert_eq!(stats.max_bytes, 300);
    assert_eq!(stats.total_bytes, 600);
    assert!((stats.mean_bytes - 200.0).abs() < 0.1);
    assert!((stats.median_bytes - 200.0).abs() < 0.1);
}

#[test]
fn test_percentiles() {
    let mut recorder = LatencyRecorder::new();
    
    for i in 1..=1000 {
        recorder.record_nanos(i * 1000);
    }
    
    let stats = recorder.finalize();
    
    assert_eq!(stats.sample_count, 1000);
    assert!(stats.median_ns > 400_000 && stats.median_ns < 600_000);
    assert!(stats.p99_ns > 980_000 && stats.p99_ns < 1_000_000);
    assert!(stats.p999_ns > 998_000);
}

#[test]
fn test_tail_amplification() {
    let mut recorder = LatencyRecorder::new();
    
    for _ in 0..500 {
        recorder.record_nanos(100_000);
    }
    for _ in 0..499 {
        recorder.record_nanos(110_000);
    }
    recorder.record_nanos(1_000_000);
    
    let stats = recorder.finalize();
    
    assert_eq!(stats.sample_count, 1000);
    assert!(stats.median_ns <= 110_000);
    assert!(stats.p9999_ns >= 1_000_000);
    assert!(stats.tail_amplification_p9999 > 5.0, "Expected tail_amplification_p9999 > 5.0, got {}", stats.tail_amplification_p9999);
    assert!(stats.tail_amplification_p999 >= 1.0);
    assert!(stats.tail_amplification_p99 >= 1.0);
}

#[test]
fn test_jitter_coefficient() {
    let mut low_jitter = LatencyRecorder::new();
    for _ in 0..1000 {
        low_jitter.record_nanos(100_000);
        low_jitter.record_nanos(101_000);
    }
    
    let mut high_jitter = LatencyRecorder::new();
    for i in 0..1000 {
        high_jitter.record_nanos(50_000 + (i * 100));
    }
    
    let low_stats = low_jitter.finalize();
    let high_stats = high_jitter.finalize();
    
    assert!(low_stats.jitter_coefficient < 0.1, "Low jitter should be < 0.1, got {}", low_stats.jitter_coefficient);
    assert!(high_stats.jitter_coefficient > 0.1, "High jitter should be > 0.1, got {}", high_stats.jitter_coefficient);
}

#[test]
fn test_lsc_computation() {
    // Values: 100, 100, 100, 100, 200
    // Median = 100
    // Absolute deviations from median: 0, 0, 0, 0, 100
    // MAD = median of deviations = 0
    // LSC = 0 / 100 = 0.0
    let mut recorder = LatencyRecorder::new_with_raw();
    for _ in 0..4 {
        recorder.record_nanos(100);
    }
    recorder.record_nanos(200);
    let stats = recorder.finalize();
    assert!(stats.lsc < 0.01, "LSC should be ~0 for concentrated data, got {}", stats.lsc);

    // Values: 10, 20, 30, 40, 50 (1000 of each for HDR accuracy)
    // Median = 30, Deviations: 20, 10, 0, 10, 20, MAD = 10
    // LSC = 10/30 ≈ 0.333
    let mut recorder2 = LatencyRecorder::new_with_raw();
    for _ in 0..1000 {
        recorder2.record_nanos(10_000);
        recorder2.record_nanos(20_000);
        recorder2.record_nanos(30_000);
        recorder2.record_nanos(40_000);
        recorder2.record_nanos(50_000);
    }
    let stats2 = recorder2.finalize();
    assert!(
        (stats2.lsc - 0.333).abs() < 0.05,
        "LSC should be ~0.333, got {}",
        stats2.lsc
    );
}

#[test]
fn test_lsc_without_raw_uses_cv() {
    // Without raw values, LSC should fall back to jitter_coefficient (CV)
    let mut recorder = LatencyRecorder::new();
    for i in 1..=1000 {
        recorder.record_nanos(i * 100);
    }
    let stats = recorder.finalize();
    assert!(
        (stats.lsc - stats.jitter_coefficient).abs() < f64::EPSILON,
        "Without raw values, LSC should equal CV"
    );
}

#[test]
fn test_roundtrip_histogram() {
    let mut encode_rec = LatencyRecorder::new();
    let mut decode_rec = LatencyRecorder::new();
    let mut rt_rec = LatencyRecorder::new_with_raw();

    for i in 0..10_000 {
        let encode_ns = 100 + (i % 50);
        let decode_ns = 200 + (i % 30);
        encode_rec.record_nanos(encode_ns);
        decode_rec.record_nanos(decode_ns);
        rt_rec.record_nanos(encode_ns + decode_ns);
    }

    let encode_stats = encode_rec.finalize();
    let decode_stats = decode_rec.finalize();
    let rt_stats = rt_rec.finalize();

    assert_eq!(rt_stats.sample_count, 10_000);
    // Round-trip median should be approximately encode_median + decode_median
    let expected_rt_median = encode_stats.median_ns + decode_stats.median_ns;
    let tolerance = expected_rt_median / 10; // 10% tolerance for HDR quantization
    assert!(
        (rt_stats.median_ns as i64 - expected_rt_median as i64).unsigned_abs() < tolerance,
        "RT median {} should be close to encode+decode median {}",
        rt_stats.median_ns,
        expected_rt_median
    );
    // RT should have LSC computed from raw values (not CV fallback)
    assert!(rt_stats.lsc >= 0.0);
}

#[test]
fn test_csv_header_column_count() {
    let header = RunResult::csv_header();
    let columns: Vec<&str> = header.split(',').collect();
    // 4 meta + 12 encode + 12 decode + 13 rt + 4 size + 2 throughput + 3 counts = 50
    assert_eq!(columns.len(), 50, "CSV header should have 50 columns, got {}", columns.len());
}

