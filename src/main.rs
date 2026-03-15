use rust_trading_serialization_bench::evaluation::metrics::RunResult;
use rust_trading_serialization_bench::evaluation::runner::{
    EvaluationConfig, EvaluationRunner, ProtocolType,
};
use rust_trading_serialization_bench::evaluation::scenarios::all_scenarios;

fn main() {
    println!("Trading Serialization Evaluation (development mode)");
    println!("Run with 'cargo run --release' for accurate measurements\n");

    let config = EvaluationConfig::default();
    let runner = EvaluationRunner::new(config);

    let protocols = ProtocolType::all();
    let scenarios = all_scenarios();
    let mut results = Vec::new();

    for scenario in &scenarios {
        println!("\n{}", "=".repeat(60));
        println!("Scenario: {} ({})", scenario.name(), scenario.short_name());
        println!("{}", "=".repeat(60));

        for protocol in &protocols {
            println!(
                "\nEvaluating {} with {}...",
                protocol.name(),
                scenario.name()
            );

            let result = runner.evaluate_single_run(*protocol, scenario, 42, 0);

            let rt = &result.roundtrip_latency;
            let rt_us = rt.median_ns as f64 / 1000.0;
            let rt_p99_us = rt.p99_ns as f64 / 1000.0;
            let rt_p999_us = rt.p999_ns as f64 / 1000.0;

            println!("  Round-trip: p50={:.2}us  p99={:.2}us  p99.9={:.2}us  TAR={:.2}x  LSC={:.4}",
                rt_us, rt_p99_us, rt_p999_us,
                rt.tail_amplification_p99, rt.lsc,
            );
            println!(
                "  Size: {:.0}B median  Throughput: {:.0} msg/s  ({:.2} MB/s)",
                result.message_size.median_bytes,
                result.throughput_msg_per_sec,
                result.throughput_bytes_per_sec / 1_000_000.0,
            );

            results.push(result);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("CSV Export (new format)");
    println!("{}", "=".repeat(60));
    println!("{}", RunResult::csv_header());
    for result in &results {
        println!("{}", result.to_csv_row());
    }
}
