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
