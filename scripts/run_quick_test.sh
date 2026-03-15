#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Quick Test: 2 protocols × 2 scenarios × 3 seeds = 12 runs
# For development validation. Takes ~2-5 minutes.
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

exec "$SCRIPT_DIR/run_experiment.sh" \
  --protocols "json rkyv" \
  --scenarios "tick book_small" \
  --seeds 42 44 \
  "$@"
