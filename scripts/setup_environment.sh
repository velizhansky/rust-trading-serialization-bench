#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# Benchmark Environment Setup (Section IV-D)
#
# Run once on a fresh AWS c5.metal (Ubuntu 24.04) instance.
# Configures CPU governor, Turbo Boost, interrupt isolation,
# installs Rust, and builds the project in release mode.
#
# Requires root/sudo for hardware configuration steps.
# Idempotent — safe to run multiple times.
#
# Usage:
#   sudo bash scripts/setup_environment.sh
# ============================================================

BENCH_CORE=2  # Must match CPU_CORE in run_experiment.sh (Section IV-D.1)

echo "=== Benchmark Environment Setup ==="
echo ""

# --- 1. System updates & essentials ---
echo "[1/7] Installing system packages..."
if command -v apt-get &>/dev/null; then
    sudo apt-get update -qq
    # flatbuffers-compiler provides 'flatc' needed by build.rs
    # linux-tools provides 'perf' (optional, for profiling)
    sudo apt-get install -y -qq build-essential flatbuffers-compiler \
        linux-tools-common linux-tools-"$(uname -r)" curl git python3 python3-pip 2>/dev/null || \
        sudo apt-get install -y -qq build-essential flatbuffers-compiler curl git python3 python3-pip
    echo "  System packages installed"
else
    echo "  WARNING: apt-get not found. Install build-essential, curl, git manually."
fi

# --- 2. Install Rust (if not present) ---
if ! command -v rustc &>/dev/null; then
    echo "[2/7] Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
    echo "  Rust installed: $(rustc --version)"
else
    echo "[2/7] Rust already installed: $(rustc --version)"
fi

# --- 3. CPU governor → performance (Section IV-D.2) ---
echo "[3/7] Setting CPU governor to 'performance'..."
GOVERNOR_PATH="/sys/devices/system/cpu/cpu${BENCH_CORE}/cpufreq/scaling_governor"
if [[ -f "$GOVERNOR_PATH" ]]; then
    CURRENT=$(cat "$GOVERNOR_PATH")
    if [[ "$CURRENT" != "performance" ]]; then
        # Set all cores — some systems require uniform governor setting
        for gov in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
            echo performance | sudo tee "$gov" > /dev/null
        done
        echo "  Governor changed: $CURRENT -> performance"
    else
        echo "  Governor already 'performance'"
    fi
else
    echo "  WARNING: Governor path not found. DVFS control unavailable."
fi

# --- 4. Disable Turbo Boost (Section IV-D.2) ---
echo "[4/7] Disabling Turbo Boost..."
TURBO_PSTATE="/sys/devices/system/cpu/intel_pstate/no_turbo"
TURBO_CPUFREQ="/sys/devices/system/cpu/cpufreq/boost"
if [[ -f "$TURBO_PSTATE" ]]; then
    CURRENT=$(cat "$TURBO_PSTATE")
    if [[ "$CURRENT" != "1" ]]; then
        echo 1 | sudo tee "$TURBO_PSTATE" > /dev/null
        echo "  Turbo Boost disabled (intel_pstate)"
    else
        echo "  Turbo Boost already disabled"
    fi
elif [[ -f "$TURBO_CPUFREQ" ]]; then
    echo 0 | sudo tee "$TURBO_CPUFREQ" > /dev/null
    echo "  Turbo Boost disabled (cpufreq/boost)"
else
    echo "  WARNING: Turbo Boost control not found. May still be active."
fi

# --- 5. Disable irqbalance (reduces interrupt migration to bench core) ---
echo "[5/7] Disabling irqbalance..."
if command -v systemctl &>/dev/null && systemctl is-active --quiet irqbalance 2>/dev/null; then
    sudo systemctl stop irqbalance
    sudo systemctl disable irqbalance
    echo "  irqbalance stopped and disabled"
elif command -v systemctl &>/dev/null; then
    echo "  irqbalance already inactive"
else
    echo "  WARNING: systemctl not found. Check irqbalance manually."
fi

# --- 6. Build the benchmark (release mode) ---
echo "[6/7] Building benchmark (release mode)..."
if [[ -f "Cargo.toml" ]]; then
    cargo build --release 2>&1 | tail -1
    echo "  Build complete"
else
    echo "  WARNING: Cargo.toml not found. Run this script from the project root."
fi

# --- 7. Verify ---
echo "[7/7] Verifying environment..."
echo ""
echo "=== Environment Summary ==="

# CPU info (Linux / macOS fallback)
if [[ -f /proc/cpuinfo ]]; then
    echo "  CPU:        $(grep 'model name' /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)"
    echo "  Cores:      $(nproc) logical"
else
    echo "  CPU:        $(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo 'unknown')"
fi

# Memory
if [[ -f /proc/meminfo ]]; then
    echo "  Memory:     $(awk '/MemTotal/ {printf "%.0f GB", $2/1024/1024}' /proc/meminfo)"
else
    echo "  Memory:     $(sysctl -n hw.memsize 2>/dev/null | awk '{printf "%.0f GB", $1/1024/1024/1024}' || echo 'unknown')"
fi

# OS
if [[ -f /etc/os-release ]]; then
    echo "  OS:         $(grep PRETTY_NAME /etc/os-release | cut -d= -f2 | tr -d '"')"
else
    echo "  OS:         $(sw_vers -productName 2>/dev/null || echo 'unknown') $(sw_vers -productVersion 2>/dev/null || echo '')"
fi

echo "  Kernel:     $(uname -r)"
echo "  Rust:       $(rustc --version 2>/dev/null || echo 'not found')"

# Governor
if [[ -f "$GOVERNOR_PATH" ]]; then
    echo "  Governor:   $(cat "$GOVERNOR_PATH")"
fi

# Turbo Boost
if [[ -f "$TURBO_PSTATE" ]]; then
    TURBO_VAL=$(cat "$TURBO_PSTATE")
    if [[ "$TURBO_VAL" == "1" ]]; then
        echo "  Turbo:      DISABLED (good)"
    else
        echo "  Turbo:      ENABLED (WARNING)"
    fi
fi

echo "  Bench core: $BENCH_CORE"

if command -v systemctl &>/dev/null; then
    echo "  irqbalance: $(systemctl is-active irqbalance 2>/dev/null || echo 'inactive')"
fi

echo ""

# Final warnings
WARNINGS=0
if [[ -f "$GOVERNOR_PATH" ]] && [[ "$(cat "$GOVERNOR_PATH")" != "performance" ]]; then
    echo "WARNING: Governor is NOT 'performance'"
    WARNINGS=$((WARNINGS + 1))
fi
if [[ -f "$TURBO_PSTATE" ]] && [[ "$(cat "$TURBO_PSTATE")" != "1" ]]; then
    echo "WARNING: Turbo Boost is NOT disabled"
    WARNINGS=$((WARNINGS + 1))
fi
if command -v systemctl &>/dev/null && [[ "$(systemctl is-active irqbalance 2>/dev/null)" == "active" ]]; then
    echo "WARNING: irqbalance is still active"
    WARNINGS=$((WARNINGS + 1))
fi

if [[ $WARNINGS -eq 0 ]]; then
    echo "Environment ready for benchmarking."
else
    echo ""
    echo "$WARNINGS warning(s). Results may have higher variance."
fi
