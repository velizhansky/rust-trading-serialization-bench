#!/usr/bin/env python3
"""
Main analysis pipeline for serialization benchmark results.

Usage:
    python analysis/analyze.py --input results/XXXXXXXX/all_runs.csv --output analysis/output/

Steps:
    1. Load and validate all_runs.csv
    2. Compute aggregated metrics with BCa CI (10,000 resamples)
    3. Normalize and compute composite scores for 3 profiles
    4. Run directional sweep sensitivity analysis
    5. Run Monte Carlo robustness (10,000 Dirichlet samples)
    6. Generate LaTeX tables
    7. Generate plots (if matplotlib available)
    8. Print summary report
"""

import argparse
import os
import sys

import numpy as np
import pandas as pd

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from lib.bootstrap import bca_bootstrap_ci
from lib.scoring import (
    METRIC_NAMES,
    PROTOCOL_ORDER,
    SCENARIO_ORDER,
    PROFILES,
    load_runs,
    validate_runs,
    compute_aggregated_results,
    normalize_metrics,
    compute_all_profile_scores,
)
from lib.sensitivity import directional_sweep, monte_carlo_robustness
from lib.tables import (
    PROTOCOL_DISPLAY,
    SCENARIO_DISPLAY,
    generate_scenario_table,
    generate_composite_score_table,
    generate_monte_carlo_table,
)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Analyze serialization benchmark results"
    )
    parser.add_argument(
        "--input", required=True, help="Path to all_runs.csv"
    )
    parser.add_argument(
        "--output", default="analysis/output", help="Output directory"
    )
    parser.add_argument(
        "--resamples",
        type=int,
        default=10_000,
        help="BCa bootstrap resamples (default: 10000)",
    )
    parser.add_argument(
        "--mc-samples",
        type=int,
        default=10_000,
        help="Monte Carlo Dirichlet samples (default: 10000)",
    )
    parser.add_argument(
        "--no-plots", action="store_true", help="Skip plot generation"
    )
    return parser.parse_args()


def print_summary(results, profile_scores, win_fractions):
    """Print human-readable summary to stdout."""
    print("\n" + "=" * 70)
    print("ANALYSIS SUMMARY")
    print("=" * 70)

    # Per-scenario best protocol (by composite score, balanced profile)
    balanced = profile_scores["balanced"]
    print("\n--- Best Protocol per Scenario (Balanced Profile) ---")
    for scenario in SCENARIO_ORDER:
        scenario_scores = [
            (p, balanced.get((p, scenario), float("inf")))
            for p in PROTOCOL_ORDER
            if (p, scenario) in balanced
        ]
        if not scenario_scores:
            continue
        best = min(scenario_scores, key=lambda x: x[1])
        disp = SCENARIO_DISPLAY.get(scenario, scenario)
        print(f"  {disp:20s}  {PROTOCOL_DISPLAY.get(best[0], best[0]):12s}  (CS={best[1]:.3f})")

    # Overall ranking by mean rank (balanced)
    print("\n--- Overall Ranking (Balanced, Mean Rank) ---")
    rank_sums = {}
    for protocol in PROTOCOL_ORDER:
        ranks = []
        for scenario in SCENARIO_ORDER:
            scenario_scores = sorted(
                [
                    (p, balanced.get((p, scenario), float("inf")))
                    for p in PROTOCOL_ORDER
                    if (p, scenario) in balanced
                ],
                key=lambda x: x[1],
            )
            rank = next(
                (i + 1 for i, (p, _) in enumerate(scenario_scores) if p == protocol),
                None,
            )
            if rank is not None:
                ranks.append(rank)
        if ranks:
            rank_sums[protocol] = sum(ranks) / len(ranks)

    for protocol, mean_rank in sorted(rank_sums.items(), key=lambda x: x[1]):
        print(f"  {PROTOCOL_DISPLAY.get(protocol, protocol):12s}  mean_rank={mean_rank:.2f}")

    # Monte Carlo summary
    print("\n--- Monte Carlo Robustness (Overall Win %) ---")
    for protocol in PROTOCOL_ORDER:
        if protocol not in win_fractions:
            continue
        fracs = win_fractions[protocol]
        overall = np.mean(list(fracs.values())) * 100
        print(f"  {PROTOCOL_DISPLAY.get(protocol, protocol):12s}  {overall:.1f}%")


def generate_plots(results, df, profile_scores, sweep_result, win_fractions, output_dir):
    """Generate all plots. Gracefully skip if matplotlib unavailable."""
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not available, skipping plots.")
        return

    plots_dir = os.path.join(output_dir, "plots")
    os.makedirs(plots_dir, exist_ok=True)

    # Colorblind-friendly palette (Wong palette)
    COLORS = {
        "json": "#E69F00",
        "bincode": "#56B4E9",
        "rkyv": "#009E73",
        "protobuf": "#F0E442",
        "flatbuffers": "#CC79A7",
    }

    protocols_present = [p for p in PROTOCOL_ORDER if any(r.protocol == p for r in results)]
    scenarios_present = [s for s in SCENARIO_ORDER if any(r.scenario == s for r in results)]

    # --- Plot 1: Tail latency profile (p50/p99/p99.9/p99.99) per scenario ---
    for scenario in scenarios_present:
        fig, ax = plt.subplots(figsize=(10, 5))
        scenario_results = [r for r in results if r.scenario == scenario]
        scenario_results.sort(
            key=lambda r: PROTOCOL_ORDER.index(r.protocol)
            if r.protocol in PROTOCOL_ORDER
            else 99
        )

        x = np.arange(len(scenario_results))
        width = 0.2
        percentiles = [
            ("p50", lambda r: r.rt_p50_ns),
            ("p99", lambda r: r.tlp),
            ("p99.9", lambda r: r.rt_p999_ns),
            ("p99.99", lambda r: r.rt_p9999_ns),
        ]

        for i, (label, getter) in enumerate(percentiles):
            vals = [getter(r).median / 1000.0 for r in scenario_results]
            ci_lo = [getter(r).ci_low / 1000.0 for r in scenario_results]
            ci_hi = [getter(r).ci_high / 1000.0 for r in scenario_results]
            yerr_lo = [v - lo for v, lo in zip(vals, ci_lo)]
            yerr_hi = [hi - v for v, hi in zip(vals, ci_hi)]
            ax.bar(
                x + i * width,
                vals,
                width,
                label=label,
                yerr=[yerr_lo, yerr_hi],
                capsize=2,
                alpha=0.85,
            )

        ax.set_ylabel("Latency (μs)")
        ax.set_title(f"Tail Latency Profile — {SCENARIO_DISPLAY.get(scenario, scenario)}")
        ax.set_xticks(x + 1.5 * width)
        ax.set_xticklabels(
            [PROTOCOL_DISPLAY.get(r.protocol, r.protocol) for r in scenario_results],
            rotation=15,
        )
        ax.legend()
        ax.set_yscale("log")
        fig.tight_layout()
        fig.savefig(
            os.path.join(plots_dir, f"tail_latency_{scenario}.png"), dpi=300
        )
        fig.savefig(
            os.path.join(plots_dir, f"tail_latency_{scenario}.pdf")
        )
        plt.close(fig)

    # --- Plot 2: TAR comparison across scenarios ---
    fig, ax = plt.subplots(figsize=(12, 5))
    x = np.arange(len(scenarios_present))
    width = 0.15
    for i, protocol in enumerate(protocols_present):
        vals = []
        errs_lo = []
        errs_hi = []
        for scenario in scenarios_present:
            r = next(
                (r for r in results if r.protocol == protocol and r.scenario == scenario),
                None,
            )
            if r:
                vals.append(r.tar.median)
                errs_lo.append(r.tar.median - r.tar.ci_low)
                errs_hi.append(r.tar.ci_high - r.tar.median)
            else:
                vals.append(np.nan)
                errs_lo.append(0)
                errs_hi.append(0)
        ax.bar(
            x + i * width,
            vals,
            width,
            label=PROTOCOL_DISPLAY.get(protocol, protocol),
            color=COLORS.get(protocol, "#999999"),
            yerr=[errs_lo, errs_hi],
            capsize=2,
        )
    ax.set_ylabel("TAR (p99/p50)")
    ax.set_title("Tail Amplification Ratio by Scenario")
    ax.set_xticks(x + width * (len(protocols_present) - 1) / 2)
    ax.set_xticklabels(
        [SCENARIO_DISPLAY.get(s, s) for s in scenarios_present], rotation=15
    )
    ax.legend(loc="upper right")
    fig.tight_layout()
    fig.savefig(os.path.join(plots_dir, "tar_comparison.png"), dpi=300)
    fig.savefig(os.path.join(plots_dir, "tar_comparison.pdf"))
    plt.close(fig)

    # --- Plot 3: Directional sweep heatmap ---
    if sweep_result:
        w_vals = sweep_result["w_tlp_values"]
        winners = sweep_result["winners"]

        # Build matrix: rows=scenarios, cols=w_tlp values
        protocol_to_idx = {p: i for i, p in enumerate(protocols_present)}
        matrix = np.zeros((len(scenarios_present), len(w_vals)))
        for si, scenario in enumerate(scenarios_present):
            for wi, winner in enumerate(winners.get(scenario, [])):
                matrix[si, wi] = protocol_to_idx.get(winner, -1)

        fig, ax = plt.subplots(figsize=(10, 5))
        cmap = plt.colormaps.get_cmap("Set2").resampled(len(protocols_present))
        im = ax.imshow(matrix, aspect="auto", cmap=cmap, interpolation="nearest")

        ax.set_xticks(range(len(w_vals)))
        ax.set_xticklabels([f"{w:.2f}" for w in w_vals], rotation=45, fontsize=8)
        ax.set_yticks(range(len(scenarios_present)))
        ax.set_yticklabels(
            [SCENARIO_DISPLAY.get(s, s) for s in scenarios_present], fontsize=9
        )
        ax.set_xlabel("w_TLP")
        ax.set_title("Directional Sweep: Winning Protocol")

        # Legend
        handles = [
            plt.Line2D(
                [0], [0],
                marker="s",
                color="w",
                markerfacecolor=cmap(i),
                markersize=10,
                label=PROTOCOL_DISPLAY.get(p, p),
            )
            for i, p in enumerate(protocols_present)
        ]
        ax.legend(handles=handles, loc="upper right", fontsize=8)
        fig.tight_layout()
        fig.savefig(os.path.join(plots_dir, "directional_sweep.png"), dpi=300)
        fig.savefig(os.path.join(plots_dir, "directional_sweep.pdf"))
        plt.close(fig)

    # --- Plot 4: Monte Carlo robustness stacked bar ---
    if win_fractions:
        fig, ax = plt.subplots(figsize=(10, 5))
        bottoms = np.zeros(len(scenarios_present))

        for protocol in protocols_present:
            fracs = [
                win_fractions.get(protocol, {}).get(s, 0) * 100
                for s in scenarios_present
            ]
            ax.bar(
                range(len(scenarios_present)),
                fracs,
                bottom=bottoms,
                label=PROTOCOL_DISPLAY.get(protocol, protocol),
                color=COLORS.get(protocol, "#999999"),
            )
            bottoms += np.array(fracs)

        ax.set_ylabel("Win Fraction (%)")
        ax.set_title("Monte Carlo Robustness (10,000 Dirichlet Samples)")
        ax.set_xticks(range(len(scenarios_present)))
        ax.set_xticklabels(
            [SCENARIO_DISPLAY.get(s, s) for s in scenarios_present], rotation=15
        )
        ax.legend(loc="upper right")
        ax.set_ylim(0, 100)
        fig.tight_layout()
        fig.savefig(os.path.join(plots_dir, "monte_carlo_robustness.png"), dpi=300)
        fig.savefig(os.path.join(plots_dir, "monte_carlo_robustness.pdf"))
        plt.close(fig)

    # --- Plot 5: Complexity scaling (OrderBook S3→S4→S5) ---
    ob_scenarios = ["book_small", "book_medium", "book_large"]
    ob_present = [s for s in ob_scenarios if s in scenarios_present]
    if len(ob_present) >= 2:
        fig, ax = plt.subplots(figsize=(8, 5))
        ob_labels = {"book_small": "5 levels", "book_medium": "20 levels", "book_large": "100 levels"}
        x_labels = [ob_labels.get(s, s) for s in ob_present]

        for protocol in protocols_present:
            vals = []
            for scenario in ob_present:
                r = next(
                    (r for r in results if r.protocol == protocol and r.scenario == scenario),
                    None,
                )
                if r:
                    vals.append(r.tlp.median / 1000.0)
                else:
                    vals.append(np.nan)
            ax.plot(
                x_labels,
                vals,
                "o-",
                label=PROTOCOL_DISPLAY.get(protocol, protocol),
                color=COLORS.get(protocol, "#999999"),
                linewidth=2,
                markersize=6,
            )

        ax.set_ylabel("Round-trip p99 (μs)")
        ax.set_xlabel("OrderBook Depth")
        ax.set_title("Tail Latency Scaling with Message Complexity (RQ2)")
        ax.legend()
        ax.set_yscale("log")
        fig.tight_layout()
        fig.savefig(os.path.join(plots_dir, "complexity_scaling.png"), dpi=300)
        fig.savefig(os.path.join(plots_dir, "complexity_scaling.pdf"))
        plt.close(fig)

    print(f"Plots saved to {plots_dir}/")


def main():
    args = parse_args()

    # Create output directories
    tables_dir = os.path.join(args.output, "tables")
    os.makedirs(tables_dir, exist_ok=True)

    # Step 1: Load and validate
    print("Loading data...")
    df = load_runs(args.input)
    print(f"  Loaded {len(df)} rows")

    warnings = validate_runs(df)
    if warnings:
        print("Validation warnings:")
        for w in warnings:
            print(f"  {w}")
    else:
        print("  Validation OK: all expected rows present")

    protocols_present = [
        p for p in PROTOCOL_ORDER if p in df["protocol"].unique()
    ]
    scenarios_present = [
        s for s in SCENARIO_ORDER if s in df["scenario"].unique()
    ]

    # Step 2: Aggregated metrics with BCa CI
    print(f"\nComputing aggregated metrics (BCa, {args.resamples} resamples)...")
    results = compute_aggregated_results(df, n_resamples=args.resamples)
    print(f"  Computed {len(results)} (protocol, scenario) aggregations")

    # Step 3: Normalization and composite scores
    print("\nComputing normalization and composite scores...")
    normalized = normalize_metrics(results)
    profile_scores = compute_all_profile_scores(normalized)

    # Step 4: Directional sweep
    print("\nRunning directional sweep (w_TLP: 0.0 → 0.6)...")
    sweep_result = directional_sweep(
        normalized, scenarios_present, protocols_present
    )

    # Step 5: Monte Carlo robustness
    print(f"\nRunning Monte Carlo robustness ({args.mc_samples} Dirichlet samples)...")
    win_fractions = monte_carlo_robustness(
        normalized, scenarios_present, protocols_present, n_samples=args.mc_samples
    )

    # Step 6: LaTeX tables
    print("\nGenerating LaTeX tables...")
    for scenario in scenarios_present:
        table = generate_scenario_table(results, scenario)
        path = os.path.join(tables_dir, f"results_{scenario}.tex")
        with open(path, "w") as f:
            f.write(table)

    composite_table = generate_composite_score_table(profile_scores)
    with open(os.path.join(tables_dir, "composite_scores.tex"), "w") as f:
        f.write(composite_table)

    mc_table = generate_monte_carlo_table(win_fractions)
    with open(os.path.join(tables_dir, "monte_carlo.tex"), "w") as f:
        f.write(mc_table)

    print(f"  Tables saved to {tables_dir}/")

    # Step 7: Plots
    if not args.no_plots:
        print("\nGenerating plots...")
        generate_plots(
            results, df, profile_scores, sweep_result, win_fractions, args.output
        )

    # Step 8: Summary
    print_summary(results, profile_scores, win_fractions)

    # Save aggregated CSV
    agg_rows = []
    for r in results:
        agg_rows.append({
            "protocol": r.protocol,
            "scenario": r.scenario,
            "tlp_median_ns": r.tlp.median,
            "tlp_ci_low": r.tlp.ci_low,
            "tlp_ci_high": r.tlp.ci_high,
            "tar_median": r.tar.median,
            "tar_ci_low": r.tar.ci_low,
            "tar_ci_high": r.tar.ci_high,
            "lsc_median": r.lsc.median,
            "lsc_ci_low": r.lsc.ci_low,
            "lsc_ci_high": r.lsc.ci_high,
            "se_median": r.se.median,
            "se_ci_low": r.se.ci_low,
            "se_ci_high": r.se.ci_high,
            "tp_median": r.tp.median,
            "tp_ci_low": r.tp.ci_low,
            "tp_ci_high": r.tp.ci_high,
            "rt_p50_median_ns": r.rt_p50_ns.median,
            "rt_p999_median_ns": r.rt_p999_ns.median,
            "rt_p9999_median_ns": r.rt_p9999_ns.median,
        })
    agg_df = pd.DataFrame(agg_rows)
    agg_path = os.path.join(args.output, "aggregated_metrics.csv")
    agg_df.to_csv(agg_path, index=False)
    print(f"\nAggregated metrics saved to {agg_path}")

    print("\nDone.")


if __name__ == "__main__":
    main()
