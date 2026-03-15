"""
LaTeX table generation for IEEE Access format.
"""

from .scoring import (
    ProtocolScenarioResult,
    AggregatedMetric,
    PROTOCOL_ORDER,
    SCENARIO_ORDER,
    PROFILES,
)


# Display names
PROTOCOL_DISPLAY = {
    "json": "JSON",
    "bincode": "Bincode",
    "rkyv": "rkyv",
    "protobuf": "Protobuf",
    "flatbuffers": "FlatBuffers",
}

SCENARIO_DISPLAY = {
    "tick": "S1: Tick",
    "order": "S2: Order",
    "book_small": "S3: OB-5",
    "book_medium": "S4: OB-20",
    "book_large": "S5: OB-100",
    "mixed": "S6: Mixed",
    "burst": "S7: Burst",
}

SCENARIO_SHORT = {
    "tick": "S1",
    "order": "S2",
    "book_small": "S3",
    "book_medium": "S4",
    "book_large": "S5",
    "mixed": "S6",
    "burst": "S7",
}


def _fmt_latency_us(m: AggregatedMetric) -> str:
    """Format latency as microseconds with CI."""
    val = m.median / 1000.0
    lo = m.ci_low / 1000.0
    hi = m.ci_high / 1000.0
    if val < 1.0:
        return f"{val:.3f} [{lo:.3f}, {hi:.3f}]"
    elif val < 100.0:
        return f"{val:.2f} [{lo:.2f}, {hi:.2f}]"
    else:
        return f"{val:.1f} [{lo:.1f}, {hi:.1f}]"


def _fmt_ratio(m: AggregatedMetric) -> str:
    """Format dimensionless ratio with CI."""
    return f"{m.median:.2f} [{m.ci_low:.2f}, {m.ci_high:.2f}]"


def _fmt_ratio_short(m: AggregatedMetric) -> str:
    """Format ratio without CI for compact tables."""
    return f"{m.median:.3f}"


def _fmt_size(m: AggregatedMetric) -> str:
    """Format size in bytes."""
    return f"{m.median:.0f}"


def _fmt_throughput(m: AggregatedMetric) -> str:
    """Format throughput as Kmsg/s with CI."""
    val = m.median / 1000.0
    lo = m.ci_low / 1000.0
    hi = m.ci_high / 1000.0
    if val >= 1000:
        return f"{val:.0f} [{lo:.0f}, {hi:.0f}]"
    else:
        return f"{val:.1f} [{lo:.1f}, {hi:.1f}]"


def generate_scenario_table(
    results: list[ProtocolScenarioResult],
    scenario: str,
) -> str:
    """Generate LaTeX table for a single scenario."""
    scenario_results = [r for r in results if r.scenario == scenario]
    # Sort by protocol order
    order = {p: i for i, p in enumerate(PROTOCOL_ORDER)}
    scenario_results.sort(key=lambda r: order.get(r.protocol, 99))

    disp = SCENARIO_DISPLAY.get(scenario, scenario)

    lines = []
    lines.append(r"\begin{table}[htbp]")
    lines.append(r"\centering")
    lines.append(f"\\caption{{Results for {disp}}}")
    lines.append(f"\\label{{tab:results_{scenario}}}")
    lines.append(r"\begin{tabular}{l r r r r r r}")
    lines.append(r"\hline")
    lines.append(
        r"Protocol & RT p50 ($\mu$s) & RT p99 ($\mu$s) & TAR & LSC & Size (B) & TP (Kmsg/s) \\"
    )
    lines.append(r"\hline")

    for r in scenario_results:
        name = PROTOCOL_DISPLAY.get(r.protocol, r.protocol)
        p50 = _fmt_latency_us(r.rt_p50_ns)
        p99 = _fmt_latency_us(r.tlp)
        tar = _fmt_ratio(r.tar)
        lsc = _fmt_ratio_short(r.lsc)
        size = _fmt_size(r.se)
        tp = _fmt_throughput(r.tp)
        lines.append(f"  {name} & {p50} & {p99} & {tar} & {lsc} & {size} & {tp} \\\\")

    lines.append(r"\hline")
    lines.append(r"\end{tabular}")
    lines.append(r"\end{table}")

    return "\n".join(lines)


def generate_composite_score_table(
    profile_scores: dict[str, dict[tuple[str, str], float]],
    rankings: dict[str, dict[str, dict[str, int]]] | None = None,
) -> str:
    """Generate LaTeX table of composite scores for all profiles."""
    lines = []
    lines.append(r"\begin{table*}[htbp]")
    lines.append(r"\centering")
    lines.append(r"\caption{Composite Scores by Usage Profile}")
    lines.append(r"\label{tab:composite_scores}")

    for profile_name, scores in profile_scores.items():
        profile_display = profile_name.replace("_", "-").title()
        weights = PROFILES[profile_name]
        weight_str = ", ".join(
            f"$w_{{\\mathrm{{{m}}}}}={v:.2f}$" for m, v in weights.items()
        )

        cols = "l" + "r" * len(SCENARIO_ORDER) + "r"
        lines.append(f"\\begin{{tabular}}{{{cols}}}")
        lines.append(r"\hline")

        header_scenarios = " & ".join(
            SCENARIO_SHORT.get(s, s) for s in SCENARIO_ORDER
        )
        lines.append(
            f"\\multicolumn{{{len(SCENARIO_ORDER) + 2}}}{{l}}"
            f"{{\\textbf{{{profile_display}}} ({weight_str})}} \\\\"
        )
        lines.append(r"\hline")
        lines.append(f"Protocol & {header_scenarios} & Mean Rank \\\\")
        lines.append(r"\hline")

        protocols = sorted(
            set(p for p, _ in scores.keys()),
            key=lambda p: PROTOCOL_ORDER.index(p) if p in PROTOCOL_ORDER else 99,
        )

        for protocol in protocols:
            name = PROTOCOL_DISPLAY.get(protocol, protocol)
            vals = []
            rank_sum = 0
            rank_count = 0

            for scenario in SCENARIO_ORDER:
                key = (protocol, scenario)
                if key in scores:
                    score = scores[key]
                    vals.append(f"{score:.3f}")

                    # Compute rank for this scenario
                    scenario_scores = sorted(
                        [
                            (p, scores[(p, scenario)])
                            for p in protocols
                            if (p, scenario) in scores
                        ],
                        key=lambda x: x[1],
                    )
                    rank = next(
                        (i + 1
                         for i, (p, _) in enumerate(scenario_scores)
                         if p == protocol),
                        None,
                    )
                    if rank is not None:
                        rank_sum += rank
                        rank_count += 1
                else:
                    vals.append("--")

            mean_rank = rank_sum / rank_count if rank_count > 0 else 0
            vals_str = " & ".join(vals)
            lines.append(f"  {name} & {vals_str} & {mean_rank:.1f} \\\\")

        lines.append(r"\hline")
        lines.append(r"\end{tabular}")
        lines.append(r"\vspace{0.5em}")
        lines.append("")

    lines.append(r"\end{table*}")
    return "\n".join(lines)


def generate_monte_carlo_table(
    win_fractions: dict[str, dict[str, float]],
) -> str:
    """Generate LaTeX table of Monte Carlo robustness results."""
    lines = []
    lines.append(r"\begin{table}[htbp]")
    lines.append(r"\centering")
    lines.append(
        r"\caption{Monte Carlo Robustness: Win Fraction (\%) over 10{,}000 Dirichlet Samples}"
    )
    lines.append(r"\label{tab:monte_carlo}")

    cols = "l" + "r" * len(SCENARIO_ORDER) + "r"
    lines.append(f"\\begin{{tabular}}{{{cols}}}")
    lines.append(r"\hline")

    header = " & ".join(SCENARIO_SHORT.get(s, s) for s in SCENARIO_ORDER)
    lines.append(f"Protocol & {header} & Overall \\\\")
    lines.append(r"\hline")

    protocols = sorted(
        win_fractions.keys(),
        key=lambda p: PROTOCOL_ORDER.index(p) if p in PROTOCOL_ORDER else 99,
    )

    for protocol in protocols:
        name = PROTOCOL_DISPLAY.get(protocol, protocol)
        vals = []
        total = 0.0
        count = 0

        for scenario in SCENARIO_ORDER:
            frac = win_fractions[protocol].get(scenario, 0.0)
            vals.append(f"{frac * 100:.1f}")
            total += frac
            count += 1

        overall = total / count * 100 if count > 0 else 0
        vals_str = " & ".join(vals)
        lines.append(f"  {name} & {vals_str} & {overall:.1f} \\\\")

    lines.append(r"\hline")
    lines.append(r"\end{tabular}")
    lines.append(r"\end{table}")

    return "\n".join(lines)
