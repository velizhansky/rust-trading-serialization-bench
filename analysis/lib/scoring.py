"""
Metric extraction, normalization, and composite scoring (Section IV-B).

Reads all_runs.csv and computes:
- Per-(protocol, scenario) aggregated metrics with BCa CIs
- Min-max normalization within each scenario
- Composite scores for three usage profiles
"""

import pandas as pd
import numpy as np
from dataclasses import dataclass, field

from .bootstrap import bca_bootstrap_ci

# Five paper metrics (Section IV-A)
METRIC_NAMES = ["TLP", "TAR", "LSC", "SE", "TP"]

# Column mapping: paper metric name -> CSV column from RunResult::csv_header().
# TAR uses the pre-computed per-run ratio (rt_tar_p99 = p99/p50 within each run),
# NOT a post-hoc ratio of aggregated percentiles — per Section IV-C.3.
METRIC_COLUMNS = {
    "TLP": "rt_p99_ns",        # Tail Latency Profile: p99 of round-trip (Section IV-A.2)
    "TAR": "rt_tar_p99",       # Tail Amplification Ratio: p99/p50 per run (Section IV-A.3)
    "LSC": "rt_lsc",           # Latency Stability Coefficient: MAD/median (Section IV-A.4)
    "SE": "size_median",       # Size Efficiency: median encoded bytes (Section IV-A.5)
    "TP": "throughput_msg_sec", # Throughput: msg/sec over 5s window (Section IV-A.6)
}

# Direction: True = lower is better, False = higher is better (Section IV-B.1)
METRIC_LOWER_IS_BETTER = {
    "TLP": True,   # lower tail latency = better
    "TAR": True,   # closer to 1.0 = more predictable
    "LSC": True,   # lower dispersion = more stable
    "SE": True,    # smaller messages = better
    "TP": False,   # higher throughput = better
}

# Usage profiles (Section IV-B.3, Table II).
# Tail-sensitive: 80% weight on tail-aware metrics (TLP+TAR+LSC).
# Balanced: equal weight — general-purpose baseline.
# Throughput-oriented: 50% TP — batch processing, market data distribution.
PROFILES = {
    "tail_sensitive": {"TLP": 0.35, "TAR": 0.25, "LSC": 0.20, "SE": 0.10, "TP": 0.10},
    "balanced": {"TLP": 0.20, "TAR": 0.20, "LSC": 0.20, "SE": 0.20, "TP": 0.20},
    "throughput_oriented": {"TLP": 0.10, "TAR": 0.10, "LSC": 0.10, "SE": 0.20, "TP": 0.50},
}

PROTOCOL_ORDER = ["json", "bincode", "rkyv", "protobuf", "flatbuffers"]
SCENARIO_ORDER = [
    "tick", "order", "book_small", "book_medium", "book_large", "mixed", "burst",
]


@dataclass
class AggregatedMetric:
    median: float
    ci_low: float
    ci_high: float


@dataclass
class ProtocolScenarioResult:
    protocol: str
    scenario: str
    tlp: AggregatedMetric = None
    tar: AggregatedMetric = None
    lsc: AggregatedMetric = None
    se: AggregatedMetric = None
    tp: AggregatedMetric = None
    # Additional diagnostics
    rt_p50_ns: AggregatedMetric = None
    rt_p999_ns: AggregatedMetric = None
    rt_p9999_ns: AggregatedMetric = None
    encode_tar_p99: AggregatedMetric = None
    decode_tar_p99: AggregatedMetric = None
    rt_cv: AggregatedMetric = None
    encode_cv: AggregatedMetric = None
    decode_cv: AggregatedMetric = None


def load_runs(csv_path: str) -> pd.DataFrame:
    """Load all_runs.csv and validate."""
    df = pd.read_csv(csv_path)
    return df


def validate_runs(df: pd.DataFrame) -> list[str]:
    """Check completeness. Returns list of warnings."""
    warnings = []

    protocols = set(df["protocol"].unique())
    scenarios = set(df["scenario"].unique())

    expected_protocols = set(PROTOCOL_ORDER)
    expected_scenarios = set(SCENARIO_ORDER)

    missing_p = expected_protocols - protocols
    if missing_p:
        warnings.append(f"Missing protocols: {missing_p}")

    missing_s = expected_scenarios - scenarios
    if missing_s:
        warnings.append(f"Missing scenarios: {missing_s}")

    # Check run counts per (protocol, scenario)
    for p in sorted(protocols):
        for s in sorted(scenarios):
            count = len(df[(df["protocol"] == p) & (df["scenario"] == s)])
            if count != 30:
                warnings.append(f"  {p}/{s}: {count} runs (expected 30)")

    total = len(df)
    expected_total = len(protocols) * len(scenarios) * 30
    if total != expected_total:
        warnings.append(f"Total rows: {total} (expected {expected_total})")

    return warnings


def _aggregate_column(
    group_df: pd.DataFrame, col: str, n_resamples: int = 10_000
) -> AggregatedMetric:
    """Aggregate a single column across runs using BCa bootstrap."""
    values = group_df[col].values
    median_val, ci_low, ci_high = bca_bootstrap_ci(
        values, n_resamples=n_resamples
    )
    return AggregatedMetric(median=median_val, ci_low=ci_low, ci_high=ci_high)


def compute_aggregated_results(
    df: pd.DataFrame, n_resamples: int = 10_000
) -> list[ProtocolScenarioResult]:
    """
    Compute aggregated metrics with BCa CI for each (protocol, scenario).
    """
    results = []

    for (protocol, scenario), group in df.groupby(["protocol", "scenario"]):
        result = ProtocolScenarioResult(
            protocol=protocol,
            scenario=scenario,
            tlp=_aggregate_column(group, "rt_p99_ns", n_resamples),
            tar=_aggregate_column(group, "rt_tar_p99", n_resamples),
            lsc=_aggregate_column(group, "rt_lsc", n_resamples),
            se=_aggregate_column(group, "size_median", n_resamples),
            tp=_aggregate_column(group, "throughput_msg_sec", n_resamples),
            # Diagnostics
            rt_p50_ns=_aggregate_column(group, "rt_p50_ns", n_resamples),
            rt_p999_ns=_aggregate_column(group, "rt_p999_ns", n_resamples),
            rt_p9999_ns=_aggregate_column(group, "rt_p9999_ns", n_resamples),
            encode_tar_p99=_aggregate_column(group, "encode_tar_p99", n_resamples),
            decode_tar_p99=_aggregate_column(group, "decode_tar_p99", n_resamples),
            rt_cv=_aggregate_column(group, "rt_cv", n_resamples),
            encode_cv=_aggregate_column(group, "encode_cv", n_resamples),
            decode_cv=_aggregate_column(group, "decode_cv", n_resamples),
        )
        results.append(result)

    return results


def _get_metric_value(result: ProtocolScenarioResult, metric: str) -> float:
    """Get the median point estimate for a metric."""
    attr = metric.lower()
    return getattr(result, attr).median


def normalize_metrics(
    results: list[ProtocolScenarioResult],
) -> dict[tuple[str, str], dict[str, float]]:
    """
    Min-max normalize metrics within each scenario (Section IV-B.1).

    Returns dict: (protocol, scenario) -> {metric_name: normalized_value}
    Normalized: 0 = best, 1 = worst.
    """
    # Group by scenario
    by_scenario: dict[str, list[ProtocolScenarioResult]] = {}
    for r in results:
        by_scenario.setdefault(r.scenario, []).append(r)

    normalized = {}

    for scenario, scenario_results in by_scenario.items():
        for metric in METRIC_NAMES:
            values = [_get_metric_value(r, metric) for r in scenario_results]
            min_val = min(values)
            max_val = max(values)
            span = max_val - min_val

            for r in scenario_results:
                val = _get_metric_value(r, metric)
                key = (r.protocol, r.scenario)
                if key not in normalized:
                    normalized[key] = {}

                if span < 1e-12:
                    # All protocols identical for this metric — no differentiation.
                    # Set to 0.0 (best) per Section IV-B.1: "contributes zero to CS."
                    normalized[key][metric] = 0.0
                elif METRIC_LOWER_IS_BETTER[metric]:
                    normalized[key][metric] = (val - min_val) / span
                else:
                    normalized[key][metric] = (max_val - val) / span

    return normalized


def compute_composite_scores(
    normalized: dict[tuple[str, str], dict[str, float]],
    weights: dict[str, float],
) -> dict[tuple[str, str], float]:
    """
    Compute composite score CS(p, s, w) = Σ w_k * norm_k(p, s).
    """
    scores = {}
    for key, norms in normalized.items():
        score = sum(weights[m] * norms[m] for m in METRIC_NAMES)
        scores[key] = score
    return scores


def compute_all_profile_scores(
    normalized: dict[tuple[str, str], dict[str, float]],
) -> dict[str, dict[tuple[str, str], float]]:
    """Compute composite scores for all three profiles."""
    return {
        profile_name: compute_composite_scores(normalized, weights)
        for profile_name, weights in PROFILES.items()
    }


def compute_rankings(
    scores: dict[tuple[str, str], float],
    scenarios: list[str] | None = None,
) -> dict[str, dict[str, int]]:
    """
    Compute per-scenario rankings (1 = best = lowest composite score).

    Returns: {protocol: {scenario: rank}}
    """
    if scenarios is None:
        scenarios = sorted(set(s for _, s in scores.keys()))

    rankings: dict[str, dict[str, int]] = {}

    for scenario in scenarios:
        scenario_scores = [
            (p, scores[(p, scenario)])
            for p, s in scores.keys()
            if s == scenario and (p, scenario) in scores
        ]
        scenario_scores.sort(key=lambda x: x[1])

        for rank, (protocol, _) in enumerate(scenario_scores, start=1):
            rankings.setdefault(protocol, {})[scenario] = rank

    return rankings
