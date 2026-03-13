use rust_trading_serialization_bench::evaluation::runner::{EvaluationRunner, EvaluationConfig};

fn main() {
    println!("Trading Serialization Evaluation");
    println!("Run with 'cargo run --release' for accurate performance measurements\n");
    
    let config = EvaluationConfig::default();
    let runner = EvaluationRunner::new(config);
    
    let results = runner.run();
    
    println!("\n{}", "=".repeat(60));
    println!("Summary");
    println!("{}", "=".repeat(60));
    
    for metrics in &results {
        metrics.print_summary();
    }
    
    println!("\n{}", "=".repeat(60));
    println!("CSV Export");
    println!("{}", "=".repeat(60));
    use rust_trading_serialization_bench::evaluation::metrics::ProtocolMetrics;
    ProtocolMetrics::print_csv_header();
    for metrics in &results {
        metrics.print_csv_row();
    }
}
