//! Single-run CLI binary for the benchmark experiment.
//!
//! Evaluates one (protocol, scenario, seed) combination and writes results
//! to CSV + JSON. Designed to be invoked by the shell orchestrator
//! (scripts/run_experiment.sh) which restarts this process between runs
//! for clean allocator state (Section IV-C.1).
//!
//! Exit codes: 0 = success, 1 = invalid arguments, 2 = benchmark error.

use rust_trading_serialization_bench::evaluation::environment::{
    capture_environment, check_environment, EnvironmentInfo,
};
use rust_trading_serialization_bench::evaluation::metrics::RunResult;
use rust_trading_serialization_bench::evaluation::runner::{
    EvaluationConfig, EvaluationRunner, ProtocolType,
};
use rust_trading_serialization_bench::evaluation::scenarios::Scenario;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process;

#[derive(Serialize)]
struct RunOutput {
    run: RunResult,
    environment: EnvironmentInfo,
}

fn print_usage() {
    eprintln!(
        "Usage: bench_single_run --protocol <name> --scenario <name> --seed <n> --run-index <n> --output-dir <path> [--quiet]"
    );
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --protocol     json|bincode|rkyv|protobuf|flatbuffers");
    eprintln!("  --scenario     tick|order|book_small|book_medium|book_large|mixed|burst");
    eprintln!("  --seed         Random seed (u64)");
    eprintln!("  --run-index    Run index (0-based)");
    eprintln!("  --output-dir   Directory to write results");
    eprintln!("  --quiet        Suppress progress output");
}

struct Args {
    protocol: ProtocolType,
    scenario: Scenario,
    seed: u64,
    run_index: usize,
    output_dir: String,
    quiet: bool,
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().collect();

    let mut protocol = None;
    let mut scenario = None;
    let mut seed = None;
    let mut run_index = None;
    let mut output_dir = None;
    let mut quiet = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--protocol" => {
                i += 1;
                let name = args.get(i).ok_or("--protocol requires a value")?;
                protocol = Some(
                    ProtocolType::from_short_name(name)
                        .ok_or_else(|| format!("Unknown protocol: '{}'", name))?,
                );
            }
            "--scenario" => {
                i += 1;
                let name = args.get(i).ok_or("--scenario requires a value")?;
                scenario = Some(
                    Scenario::from_short_name(name)
                        .ok_or_else(|| format!("Unknown scenario: '{}'", name))?,
                );
            }
            "--seed" => {
                i += 1;
                let val = args.get(i).ok_or("--seed requires a value")?;
                seed = Some(
                    val.parse::<u64>()
                        .map_err(|_| format!("Invalid seed: '{}'", val))?,
                );
            }
            "--run-index" => {
                i += 1;
                let val = args.get(i).ok_or("--run-index requires a value")?;
                run_index = Some(
                    val.parse::<usize>()
                        .map_err(|_| format!("Invalid run-index: '{}'", val))?,
                );
            }
            "--output-dir" => {
                i += 1;
                output_dir = Some(
                    args.get(i)
                        .ok_or("--output-dir requires a value")?
                        .clone(),
                );
            }
            "--quiet" => {
                quiet = true;
            }
            other => {
                return Err(format!("Unknown argument: '{}'", other));
            }
        }
        i += 1;
    }

    Ok(Args {
        protocol: protocol.ok_or("--protocol is required")?,
        scenario: scenario.ok_or("--scenario is required")?,
        seed: seed.ok_or("--seed is required")?,
        run_index: run_index.ok_or("--run-index is required")?,
        output_dir: output_dir.ok_or("--output-dir is required")?,
        quiet,
    })
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            print_usage();
            process::exit(1);
        }
    };

    // Environment checks
    let warnings = check_environment();
    for w in &warnings {
        eprintln!("{}", w);
    }

    if !args.quiet {
        eprintln!(
            "Running: protocol={} scenario={} seed={} run_index={}",
            args.protocol.short_name(),
            args.scenario.short_name(),
            args.seed,
            args.run_index,
        );
    }

    // Capture environment before benchmark (so timer measurement doesn't interfere)
    let env_info = capture_environment();

    // Run benchmark
    let config = EvaluationConfig {
        protocols: vec![args.protocol],
        scenarios: vec![args.scenario],
        baseline_protocol: args.protocol,
    };
    let runner = EvaluationRunner::new(config);

    let result = runner.evaluate_single_run(
        args.protocol,
        &args.scenario,
        args.seed,
        args.run_index,
    );

    // Ensure output directory exists
    let output_dir = Path::new(&args.output_dir);
    if let Err(e) = fs::create_dir_all(output_dir) {
        eprintln!("Error creating output directory: {}", e);
        process::exit(2);
    }

    let file_prefix = format!(
        "run_{}_{}_{}",
        args.protocol.short_name(),
        args.scenario.short_name(),
        args.seed
    );

    // Write CSV
    let csv_path = output_dir.join(format!("{}.csv", file_prefix));
    let csv_content = format!("{}\n{}\n", RunResult::csv_header(), result.to_csv_row());
    if let Err(e) = fs::write(&csv_path, &csv_content) {
        eprintln!("Error writing CSV: {}", e);
        process::exit(2);
    }

    // Write JSON
    let json_path = output_dir.join(format!("{}.json", file_prefix));
    let output = RunOutput {
        run: result.clone(),
        environment: env_info,
    };
    let json_content = match serde_json::to_string_pretty(&output) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Error serializing JSON: {}", e);
            process::exit(2);
        }
    };
    if let Err(e) = fs::write(&json_path, &json_content) {
        eprintln!("Error writing JSON: {}", e);
        process::exit(2);
    }

    // Progress output to stdout
    let rt = &result.roundtrip_latency;
    println!(
        "{}\t{}\tseed={}\trt_p50={:.2}us\trt_p99={:.2}us\trt_p999={:.2}us\ttp={:.0}msg/s",
        result.protocol_name,
        result.scenario_name,
        result.seed,
        rt.median_ns as f64 / 1000.0,
        rt.p99_ns as f64 / 1000.0,
        rt.p999_ns as f64 / 1000.0,
        result.throughput_msg_per_sec,
    );
}
