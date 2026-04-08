# Tail-Aware Evaluation of Serialization Protocols in Rust for Latency-Sensitive Trading Workloads

Supplementary code and data for the paper:

> P. Velizhansky, "Tail-Aware Evaluation of Serialization Protocols in Rust for Latency-Sensitive Trading Workloads," *IEEE Access*, 2026.

This repository contains the complete benchmark implementation, raw experimental data, and analysis pipeline required to reproduce the results reported in the paper.

## Overview

The benchmark evaluates five serialization protocols across seven trading-domain workloads, focusing on tail latency behavior (p99–p99.99) rather than mean/median performance. The evaluation uses a structured multi-criteria framework with three custom metrics: Tail Amplification Ratio (TAR), Tail Latency Profile (TLP), and Latency Stability Coefficient (LSC).

### Protocols

| Protocol | Encoding | Access Model | Rust Crate |
|----------|----------|-------------|------------|
| JSON | Text, self-describing | Traditional | `serde_json` via `serde` |
| Bincode | Binary, native layout | Traditional | `bincode` 2.x (`bincode-next`) via `serde` |
| rkyv | Binary, archived layout | Zero-copy | `rkyv` |
| Protobuf | Binary, varint-encoded | Traditional | `prost` |
| FlatBuffers | Binary, offset-based | Zero-copy | `flatbuffers` (official codegen) |

### Evaluation Scenarios

| ID | Scenario | Message Type | Messages |
|----|----------|-------------|----------|
| S1 | Tick Streaming | Tick (8 fields, fixed-size) | 1,000,000 |
| S2 | Order Entry | Order (9 fields, 2 var-length strings) | 500,000 |
| S3 | OrderBook Small | OrderBook (5 levels per side) | 100,000 |
| S4 | OrderBook Medium | OrderBook (20 levels per side) | 100,000 |
| S5 | OrderBook Large | OrderBook (70–100 levels per side) | 100,000 |
| S6 | Mixed Workload | 70% ticks, 20% orders, 7/2/1% books | 1,000,000 |
| S7 | Burst Traffic | 90% mixed + 10% tick burst | 1,000,000 |

### Experimental Design

Full factorial: **5 protocols × 7 scenarios × 30 seeds = 1,050 independent runs**. Each run is a separate OS process to ensure clean allocator state. Seeds 42–71 via ChaCha20 PRNG (`StdRng`) produce deterministic, reproducible message sequences.

## Repository Structure

```
├── src/
│   ├── messages/              # Domain types: Tick, Order, OrderBook
│   │   ├── tick.rs            # 8-field fixed-size market data message
│   │   ├── order.rs           # 9-field order with variable-length strings
│   │   └── order_book.rs      # Nested variable-depth price level arrays
│   ├── protocols/             # Encode, decode, and zero-copy access per protocol
│   │   ├── json.rs
│   │   ├── bincode.rs
│   │   ├── rkyv.rs            # Zero-copy: buffer validation + field traversal
│   │   ├── protobuf.rs
│   │   └── flatbuffers.rs     # Zero-copy: buffer validation + field traversal
│   ├── evaluation/
│   │   ├── scenarios.rs       # 7 deterministic workload generators
│   │   ├── metrics.rs         # HDR histogram, TAR, TLP, LSC, size, throughput
│   │   ├── runner.rs          # 3-phase measurement: warmup → throughput → latency
│   │   └── environment.rs     # Runtime environment capture and validation
│   ├── bin/
│   │   └── bench_single_run.rs  # CLI binary for one (protocol, scenario, seed)
│   ├── main.rs
│   └── lib.rs
├── schemas/
│   ├── trading.proto          # Protobuf schema (3 message types)
│   └── trading.fbs            # FlatBuffers schema (3 message types)
├── tests/
│   ├── serialization_correctness.rs  # Roundtrip + zero-copy access (14 tests)
│   ├── scenarios_validation.rs       # Determinism, distributions, ranges (9 tests)
│   ├── metrics_validation.rs         # TAR, LSC, percentiles, CSV format (7 tests)
│   └── runner_validation.rs          # Measurement phases, invariants (5 tests)
├── scripts/
│   ├── setup_environment.sh   # One-time server setup (governor, turbo, packages)
│   ├── run_experiment.sh      # Full 1,050-run orchestrator with CPU pinning
│   └── run_quick_test.sh      # Quick validation: 12 runs, ~2–5 minutes
├── analysis/
│   ├── analyze.py             # BCa bootstrap CI, composite scores, sensitivity
│   ├── lib/                   # Bootstrap, scoring, sensitivity, LaTeX tables
│   ├── output/                # Generated plots (.png) and tables (.tex)
│   └── requirements.txt
├── results/
│   ├── all_runs.csv           # 1,050 rows of raw metrics (52 columns per row)
│   ├── environment.json       # Captured hardware/software/config metadata
│   └── raw_runs.zip           # Per-run CSV and JSON files
├── build.rs                   # FlatBuffers/Protobuf codegen + build-time env capture
├── Cargo.toml
└── Cargo.lock                 # Pinned dependency versions for reproducibility
```

## Prerequisites

**System packages** (Ubuntu 24.04):

```bash
sudo apt install build-essential flatbuffers-compiler protobuf-compiler
```

**Rust** (stable, edition 2024):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

**Python** (for analysis only):

```bash
pip install -r analysis/requirements.txt
```

Alternatively, run the automated setup:

```bash
sudo bash scripts/setup_environment.sh
```

This installs all packages, configures the CPU governor to `performance`, disables Turbo Boost, stops `irqbalance`, and builds the project.

## Build

```bash
cargo build --release
```

The build script (`build.rs`) compiles Protobuf and FlatBuffers schemas from `schemas/` and captures compiler version, optimization level, LTO setting, and all dependency versions from `Cargo.lock` at compile time.

## Tests

```bash
cargo test
```

The test suite (35 tests) validates:

- **Serialization correctness**: roundtrip encode→decode for all 5 protocols × 3 message types, plus zero-copy access guards for rkyv and FlatBuffers
- **Scenario generation**: determinism across runs, correct message counts, mixed workload distribution within tolerance, burst traffic structure, OrderBook-Large depth range (70–100 levels), symbol/client_order_id string length ranges, shared instrument ID set across message types
- **Metrics**: percentile accuracy, tail amplification ratio computation, jitter coefficient, LSC (MAD/median) exact computation and CV fallback, round-trip histogram additivity, CSV column count consistency
- **Runner**: measurement phase ordering, amplification computation, throughput no-wrap-around invariant, corpus exhaustion flag consistency

## Reproducing the Experiment

### Full Experiment (≈6–10 hours)

```bash
# 1. Set up the benchmark environment (one-time, requires sudo)
sudo bash scripts/setup_environment.sh

# 2. Run all 1,050 benchmarks with CPU pinning to core 2
bash scripts/run_experiment.sh
```

The orchestrator:
- Runs each (protocol, scenario, seed) combination as a separate process via `taskset -c 2`
- Performs pre-flight checks: binary exists, smoke test, CPU governor, Turbo Boost
- Reports progress every 10 runs with estimated time remaining
- Merges per-run CSVs into `all_runs.csv` and copies `environment.json`
- Supports `--resume` to continue after interruption

### Quick Validation (≈2–5 minutes)

```bash
bash scripts/run_quick_test.sh
```

Runs 2 protocols × 2 scenarios × 3 seeds = 12 runs for a fast sanity check.

### Custom Runs

```bash
# Single run
taskset -c 2 ./target/release/bench_single_run \
  --protocol rkyv --scenario tick --seed 42 --run-index 0 \
  --output-dir results/custom/

# Custom subset via orchestrator
bash scripts/run_experiment.sh \
  --protocols "rkyv flatbuffers" \
  --scenarios "tick order" \
  --seeds 42 46
```

## Analysis Pipeline

```bash
python analysis/analyze.py \
  --input results/all_runs.csv \
  --output analysis/output/
```

The pipeline:
1. Loads and validates `all_runs.csv` (checks 1,050 rows, 5 protocols × 7 scenarios × 30 seeds)
2. Computes aggregated metrics with BCa bootstrap confidence intervals (10,000 resamples)
3. Normalizes metrics and computes composite scores for three decision profiles (latency-focused, balanced, size-focused)
4. Runs directional sweep sensitivity analysis
5. Runs Monte Carlo robustness check (10,000 Dirichlet weight samples)
6. Generates LaTeX tables (per-scenario results, composite scores, Monte Carlo summary)
7. Generates publication-ready plots (tail latency profiles, TAR comparison, complexity scaling, Monte Carlo robustness, directional sweep)

### Outputs

- `analysis/output/aggregated_metrics.csv` — per-protocol, per-scenario aggregated metrics with 95% CI
- `analysis/output/plots/*.png` — 11 figures used in the paper (300 DPI)
- `analysis/output/tables/*.tex` — 16 LaTeX tables included in the paper via `\input{}`

## Data Format

### `results/all_runs.csv`

Each row represents one benchmark run (52 columns):

| Columns | Description |
|---------|-------------|
| `protocol`, `scenario`, `seed`, `run_index` | Run identification |
| `encode_p50_ns` ... `encode_tar_p9999` | Encode latency distribution (12 columns) |
| `decode_p50_ns` ... `decode_tar_p9999` | Decode latency distribution (12 columns) |
| `rt_p50_ns` ... `rt_tar_p9999` | Round-trip latency distribution + LSC (13 columns) |
| `size_median` ... `size_max` | Serialized message size (4 columns) |
| `throughput_msg_sec`, `throughput_bytes_sec` | Throughput (2 columns) |
| `throughput_corpus_exhausted`, `throughput_processed` | Throughput diagnostics (2 columns) |
| `total_messages`, `warmup_messages`, `measured_messages` | Message counts (3 columns) |

### `results/environment.json`

Hardware/software metadata captured programmatically at experiment start. Includes CPU model, cache hierarchy, OS/kernel versions, Rust compiler version, optimization settings, CPU governor and Turbo Boost state, timer resolution, and exact dependency versions parsed from `Cargo.lock`.

## Measurement Procedure

Each run follows a three-phase protocol (see paper, Section III-D):

1. **Warmup** (5,000 messages) — exercises the same encode + access code path; results discarded
2. **Throughput** — single forward pass over post-warmup messages within a 5-second window; no wrap-around
3. **Latency** — per-message encode/decode/round-trip timing into three separate HDR histograms (3 significant digits)

For zero-copy protocols (rkyv, FlatBuffers), the "decode" phase performs buffer validation and full field traversal via archived references without allocating owned structures, matching real-world zero-copy usage patterns.

## Experimental Controls

- **CPU pinning**: all measurements on core 2 via `taskset`
- **CPU governor**: set to `performance` (frequency scaling disabled)
- **Turbo Boost**: disabled (constant clock frequency)
- **Process isolation**: each run is a separate OS process (clean allocator state)
- **Interrupt isolation**: `irqbalance` disabled
- **Compiler optimization**: `opt-level = 3`, LTO = thin
- **Dead code prevention**: `std::hint::black_box()` on all decode/access results
- **Deterministic PRNG**: ChaCha20 seeds 42–71, message sequences reproducible from seed + parameters

## Crate Note

The `bincode-next` crate (version 2.0.4) is the Bincode 2.x release, a ground-up rewrite of the original `bincode` crate. It is imported as `bincode-next` in `Cargo.toml` because the 2.x series was published under this name during its transition period. In the paper and throughout the codebase, it is referred to simply as "Bincode."

## Citation

```bibtex
@article{velizhansky2026tailaware,
  author    = {Velizhansky, Pavel},
  title     = {Tail-Aware Evaluation of Serialization Protocols in {Rust}
               for Latency-Sensitive Trading Workloads},
  journal   = {IEEE Access},
  year      = {2026},
  note      = {Preprint: arXiv:XXXX.XXXXX}
}
```

## License

MIT License. See [LICENSE](LICENSE).