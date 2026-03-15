use rust_trading_serialization_bench::evaluation::runner::{
    EvaluationRunner, EvaluationConfig, ProtocolType,
};
use rust_trading_serialization_bench::evaluation::scenarios::Scenario;


#[test]
fn test_runner_basic() {
    let config = EvaluationConfig {
        protocols: vec![ProtocolType::Json, ProtocolType::Bincode],
        scenarios: vec![Scenario::TickStreaming],
        baseline_protocol: ProtocolType::Json,
    };
    
    let runner = EvaluationRunner::new(config);
    let results = runner.run();
    
    assert_eq!(results.len(), 2);
    
    for metrics in &results {
        assert!(metrics.encode_latency.sample_count > 0);
        assert!(metrics.decode_latency.sample_count > 0);
        assert!(metrics.message_size.sample_count > 0);
        assert!(metrics.throughput_msg_per_sec > 0.0);
        assert!(metrics.encode_latency.median_ns > 0);
        assert!(metrics.decode_latency.median_ns > 0);
    }
}

#[test]
fn test_amplification_computed() {
    let config = EvaluationConfig {
        protocols: vec![ProtocolType::Json, ProtocolType::Bincode],
        scenarios: vec![Scenario::OrderEntry],
        baseline_protocol: ProtocolType::Json,
    };
    
    let runner = EvaluationRunner::new(config);
    let results = runner.run();
    
    let json_result = results.iter().find(|m| m.protocol_name == "JSON").unwrap();
    let bincode_result = results.iter().find(|m| m.protocol_name == "Bincode").unwrap();
    
    assert!(json_result.encode_latency_amplification.is_some());
    assert!(bincode_result.encode_latency_amplification.is_some());
    
    let json_amp = json_result.encode_latency_amplification.unwrap();
    assert!((json_amp - 1.0).abs() < 0.01);
}

#[test]
fn test_multiple_scenarios() {
    let config = EvaluationConfig {
        protocols: vec![ProtocolType::Json],
        scenarios: vec![
            Scenario::TickStreaming,
            Scenario::OrderEntry,
            Scenario::OrderBookSmall,
        ],
        baseline_protocol: ProtocolType::Json,
    };
    
    let runner = EvaluationRunner::new(config);
    let results = runner.run();
    
    assert_eq!(results.len(), 3);
    
    let scenario_names: Vec<String> = results.iter()
        .map(|m| m.scenario_name.clone())
        .collect();
    
    assert!(scenario_names.contains(&"Tick Streaming".to_string()));
    assert!(scenario_names.contains(&"Order Entry".to_string()));
    assert!(scenario_names.contains(&"OrderBook Small (5 levels)".to_string()));
}

#[test]
fn test_evaluate_single_run() {
    let config = EvaluationConfig {
        protocols: vec![ProtocolType::Json],
        scenarios: vec![Scenario::OrderBookSmall],
        baseline_protocol: ProtocolType::Json,
    };

    let runner = EvaluationRunner::new(config);
    let result = runner.evaluate_single_run(
        ProtocolType::Json,
        &Scenario::OrderBookSmall,
        42,
        0,
    );

    // Metadata
    assert_eq!(result.protocol_name, "json");
    assert_eq!(result.scenario_name, "book_small");
    assert_eq!(result.seed, 42);
    assert_eq!(result.run_index, 0);
    assert_eq!(result.total_messages, 100_000);
    assert_eq!(result.warmup_messages, 5_000);
    assert_eq!(result.measured_messages, 95_000);

    // Three latency distributions populated
    assert_eq!(result.encode_latency.sample_count, 95_000);
    assert_eq!(result.decode_latency.sample_count, 95_000);
    assert_eq!(result.roundtrip_latency.sample_count, 95_000);

    // Round-trip should have meaningful LSC (computed from raw values)
    assert!(result.roundtrip_latency.lsc >= 0.0);

    // RT median should be >= encode median + decode median (approximately)
    assert!(
        result.roundtrip_latency.median_ns >= result.encode_latency.median_ns,
        "RT median should be >= encode median"
    );

    // Size stats
    assert_eq!(result.message_size.sample_count, 95_000);
    assert!(result.message_size.median_bytes > 0.0);

    // Throughput
    assert!(result.throughput_msg_per_sec > 0.0);
    assert!(result.throughput_bytes_per_sec > 0.0);

    // CSV serialization produces correct column count
    let csv_row = result.to_csv_row();
    let columns: Vec<&str> = csv_row.split(',').collect();
    assert_eq!(
        columns.len(),
        50,
        "CSV row should have 50 columns, got {}",
        columns.len()
    );
}

#[test]
fn test_protocol_short_name_roundtrip() {
    for proto in ProtocolType::all() {
        let short = proto.short_name();
        let recovered = ProtocolType::from_short_name(short)
            .unwrap_or_else(|| panic!("from_short_name failed for '{}'", short));
        assert_eq!(proto, recovered);
    }
}
