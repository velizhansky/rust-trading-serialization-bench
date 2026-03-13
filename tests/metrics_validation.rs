use rust_trading_serialization_bench::evaluation::metrics::{
    LatencyRecorder, SizeRecorder,
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

