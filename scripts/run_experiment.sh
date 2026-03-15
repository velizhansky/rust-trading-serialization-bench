#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Full Experiment Orchestrator
# Runs 5 protocols × 7 scenarios × 30 seeds = 1,050 independent benchmark runs.
# Each run is a separate process to ensure clean allocator state (Section IV-C.1).
# CPU pinned to core 2 via taskset (Section IV-D.1).
# =============================================================================

# --- Defaults ----------------------------------------------------------------
PROTOCOLS="json bincode rkyv protobuf flatbuffers"
SCENARIOS="tick order book_small book_medium book_large mixed burst"
SEED_START=42
SEED_END=71
CPU_CORE=2
OUTPUT_DIR="results/$(date +%Y%m%d_%H%M%S)"
BINARY="./target/release/bench_single_run"
DRY_RUN=false
RESUME=false

# --- Parse CLI overrides -----------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-dir)
      OUTPUT_DIR="$2"; shift 2 ;;
    --protocols)
      PROTOCOLS="$2"; shift 2 ;;
    --scenarios)
      SCENARIOS="$2"; shift 2 ;;
    --seeds)
      SEED_START="$2"; SEED_END="$3"; shift 3 ;;
    --cpu-core)
      CPU_CORE="$2"; shift 2 ;;
    --dry-run)
      DRY_RUN=true; shift ;;
    --resume)
      RESUME=true; shift ;;
    --binary)
      BINARY="$2"; shift 2 ;;
    -h|--help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --output-dir <dir>         Override output directory"
      echo "  --protocols <list>         Space-separated protocol subset (quoted)"
      echo "  --scenarios <list>         Space-separated scenario subset (quoted)"
      echo "  --seeds <start> <end>      Override seed range (default: 42 71)"
      echo "  --cpu-core <n>             Override CPU core for pinning (default: 2)"
      echo "  --binary <path>            Override benchmark binary path"
      echo "  --dry-run                  Print commands without executing"
      echo "  --resume                   Skip already-completed runs"
      echo "  -h, --help                 Show this help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# --- Compute plan ------------------------------------------------------------
NUM_PROTOCOLS=$(echo $PROTOCOLS | wc -w | tr -d ' ')
NUM_SCENARIOS=$(echo $SCENARIOS | wc -w | tr -d ' ')
NUM_SEEDS=$(( SEED_END - SEED_START + 1 ))
TOTAL_RUNS=$(( NUM_PROTOCOLS * NUM_SCENARIOS * NUM_SEEDS ))

echo "============================================================"
echo "Serialization Benchmark Experiment"
echo "============================================================"
echo "Protocols ($NUM_PROTOCOLS): $PROTOCOLS"
echo "Scenarios ($NUM_SCENARIOS): $SCENARIOS"
echo "Seeds:      $SEED_START..$SEED_END ($NUM_SEEDS runs each)"
echo "Total runs: $TOTAL_RUNS"
echo "CPU core:   $CPU_CORE"
echo "Output:     $OUTPUT_DIR"
echo "Binary:     $BINARY"
echo "Resume:     $RESUME"
echo "============================================================"

if [[ "$DRY_RUN" == "true" ]]; then
  echo ""
  echo "[DRY RUN] Commands that would be executed:"
  echo ""
  for protocol in $PROTOCOLS; do
    for scenario in $SCENARIOS; do
      for seed in $(seq $SEED_START $SEED_END); do
        run_index=$((seed - SEED_START))
        echo "  taskset -c $CPU_CORE $BINARY --protocol $protocol --scenario $scenario --seed $seed --run-index $run_index --output-dir $OUTPUT_DIR --quiet"
      done
    done
  done
  echo ""
  echo "[DRY RUN] $TOTAL_RUNS commands total. No runs executed."
  exit 0
fi

# --- Pre-flight checks -------------------------------------------------------
echo ""
echo "Pre-flight checks..."

# 1. Binary exists (build if needed)
if [[ ! -x "$BINARY" ]]; then
  echo "  Binary not found at $BINARY, building..."
  cargo build --release
  if [[ ! -x "$BINARY" ]]; then
    echo "ERROR: Failed to build binary." >&2
    exit 1
  fi
fi
echo "  [OK] Binary: $BINARY"

# 2. Create output directory
mkdir -p "$OUTPUT_DIR"
LOG_FILE="${OUTPUT_DIR}/experiment.log"
echo "  [OK] Output directory: $OUTPUT_DIR"

# 3. Smoke test
PREFLIGHT_DIR=$(mktemp -d)
if $BINARY --protocol json --scenario tick --seed 0 --run-index 0 --output-dir "$PREFLIGHT_DIR" --quiet 2>/dev/null; then
  if [[ -f "$PREFLIGHT_DIR/run_json_tick_0.csv" && -f "$PREFLIGHT_DIR/run_json_tick_0.json" ]]; then
    echo "  [OK] Smoke test passed"
  else
    echo "ERROR: Smoke test did not produce expected output files." >&2
    rm -rf "$PREFLIGHT_DIR"
    exit 1
  fi
else
  echo "ERROR: Smoke test failed (binary returned non-zero)." >&2
  rm -rf "$PREFLIGHT_DIR"
  exit 1
fi
rm -rf "$PREFLIGHT_DIR"

# 4. CPU governor check (Linux only)
GOVERNOR_FILE="/sys/devices/system/cpu/cpu${CPU_CORE}/cpufreq/scaling_governor"
if [[ -f "$GOVERNOR_FILE" ]]; then
  GOVERNOR=$(cat "$GOVERNOR_FILE")
  if [[ "$GOVERNOR" != "performance" ]]; then
    echo "  [WARN] CPU governor is '$GOVERNOR', should be 'performance'"
  else
    echo "  [OK] CPU governor: performance"
  fi
else
  echo "  [INFO] CPU governor: cannot check (not Linux or cpufreq not available)"
fi

# 5. Turbo boost check (Linux/Intel only)
TURBO_FILE="/sys/devices/system/cpu/intel_pstate/no_turbo"
if [[ -f "$TURBO_FILE" ]]; then
  NO_TURBO=$(cat "$TURBO_FILE")
  if [[ "$NO_TURBO" == "0" ]]; then
    echo "  [WARN] Turbo Boost is ENABLED. Disable for stable measurements."
  else
    echo "  [OK] Turbo Boost: disabled"
  fi
else
  echo "  [INFO] Turbo Boost: cannot check (not Intel pstate)"
fi

# 6. Check taskset availability
if command -v taskset &>/dev/null; then
  PIN_CMD="taskset -c $CPU_CORE"
  echo "  [OK] taskset available, pinning to core $CPU_CORE"
else
  PIN_CMD=""
  echo "  [WARN] taskset not available (not Linux?), running without CPU pinning"
fi

echo ""
echo "Starting experiment at $(date -u +%Y-%m-%dT%H:%M:%SZ)..."
echo "Experiment started at $(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$LOG_FILE"
echo ""

# --- Execution loop ----------------------------------------------------------
COMPLETED=0
SKIPPED=0
FAILED=0
START_TIME=$(date +%s)

for protocol in $PROTOCOLS; do
  for scenario in $SCENARIOS; do
    for seed in $(seq $SEED_START $SEED_END); do
      run_index=$((seed - SEED_START))
      output_file="${OUTPUT_DIR}/run_${protocol}_${scenario}_${seed}.csv"

      # Resume: skip if output already exists
      if [[ "$RESUME" == "true" && -f "$output_file" ]]; then
        SKIPPED=$((SKIPPED + 1))
        COMPLETED=$((COMPLETED + 1))
        continue
      fi

      # Run with optional CPU pinning
      if $PIN_CMD $BINARY \
        --protocol "$protocol" \
        --scenario "$scenario" \
        --seed "$seed" \
        --run-index "$run_index" \
        --output-dir "$OUTPUT_DIR" \
        --quiet 2>>"$LOG_FILE"; then
        COMPLETED=$((COMPLETED + 1))
      else
        FAILED=$((FAILED + 1))
        echo "FAILED: $protocol $scenario seed=$seed" | tee -a "$LOG_FILE"
      fi

      # Progress report every 10 runs
      if (( COMPLETED % 10 == 0 && COMPLETED > 0 )); then
        NOW=$(date +%s)
        ELAPSED=$(( NOW - START_TIME ))
        if (( ELAPSED > 0 )); then
          REMAINING_RUNS=$(( TOTAL_RUNS - COMPLETED ))
          RATE=$(awk -v c="$COMPLETED" -v e="$ELAPSED" 'BEGIN {printf "%.1f", c / e}')
          ETA=$(awk -v r="$REMAINING_RUNS" -v c="$COMPLETED" -v e="$ELAPSED" 'BEGIN {printf "%.0f", r / (c / e)}')
          echo "[${COMPLETED}/${TOTAL_RUNS}] ${RATE} runs/sec, ~${ETA}s remaining, ${FAILED} failed"
        fi
      fi
    done
  done
done

END_TIME=$(date +%s)
TOTAL_TIME=$(( END_TIME - START_TIME ))

# --- Post-execution ----------------------------------------------------------
echo ""
echo "Merging CSV files..."

# Merge all per-run CSVs into all_runs.csv
FIRST_CSV=$(ls "${OUTPUT_DIR}"/run_*.csv 2>/dev/null | head -1)
if [[ -n "$FIRST_CSV" ]]; then
  head -1 "$FIRST_CSV" > "${OUTPUT_DIR}/all_runs.csv"
  for f in "${OUTPUT_DIR}"/run_*.csv; do
    tail -1 "$f" >> "${OUTPUT_DIR}/all_runs.csv"
  done
  DATA_ROWS=$(( $(wc -l < "${OUTPUT_DIR}/all_runs.csv") - 1 ))
  echo "  all_runs.csv: $DATA_ROWS data rows"
else
  echo "  WARNING: No CSV files found to merge."
fi

# Copy one JSON environment file
FIRST_JSON=$(ls "${OUTPUT_DIR}"/run_*.json 2>/dev/null | head -1)
if [[ -n "$FIRST_JSON" ]]; then
  # Extract just the environment section
  python3 -c "
import json, sys
with open('$FIRST_JSON') as f:
    data = json.load(f)
with open('${OUTPUT_DIR}/environment.json', 'w') as f:
    json.dump(data.get('environment', {}), f, indent=2)
" 2>/dev/null || cp "$FIRST_JSON" "${OUTPUT_DIR}/environment.json"
  echo "  environment.json: copied"
fi

echo ""
echo "============================================================"
echo "Experiment Complete"
echo "============================================================"
echo "  Total runs:  $TOTAL_RUNS"
echo "  Completed:   $COMPLETED"
echo "  Skipped:     $SKIPPED (resume)"
echo "  Failed:      $FAILED"
echo "  Total time:  ${TOTAL_TIME}s"
if (( TOTAL_TIME > 0 )); then
  FINAL_RATE=$(awk -v c="$COMPLETED" -v t="$TOTAL_TIME" 'BEGIN {printf "%.1f", c / t}')
  echo "  Rate:        ${FINAL_RATE} runs/sec"
fi
echo "  Output:      $OUTPUT_DIR"
echo "============================================================"

echo "Experiment completed at $(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$LOG_FILE"

# Exit with error if any runs failed
if (( FAILED > 0 )); then
  exit 1
fi
