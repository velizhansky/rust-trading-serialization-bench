"""
Sensitivity analysis (Section IV-B.4).

Two methods to assess whether protocol rankings are robust to weight choice:

1. Directional sweep: vary w_TLP from 0.0 to 0.6 in 0.05 steps (13 configs),
   remainder distributed equally among TAR, LSC, SE, TP. Shows how tail-latency
   emphasis shifts the winning protocol.

2. Monte Carlo robustness: sample 10,000 weight vectors uniformly from the
   5-simplex via symmetric Dirichlet(1,1,1,1,1). Reports the fraction of
   configurations where each protocol achieves the lowest composite score.
"""

import numpy as np

from .scoring import (
    METRIC_NAMES,
    compute_composite_scores,
)


def directional_sweep(
    normalized: dict[tuple[str, str], dict[str, float]],
    scenarios: list[str],
    protocols: list[str],
    steps: int = 13,
    w_tlp_max: float = 0.6,
) -> dict:
    """
    Vary w_TLP from 0.0 to w_tlp_max in `steps` configurations.
    Remaining weight distributed equally among TAR, LSC, SE, TP.

    Returns
    -------
    {
        "w_tlp_values": [0.0, 0.05, ...],
        "winners": {scenario: [winner_at_w0, winner_at_w1, ...]},
        "scores": {w_tlp: {(protocol, scenario): score}},
    }
    """
    w_tlp_values = [round(i * w_tlp_max / (steps - 1), 4) for i in range(steps)]

    all_scores = {}
    winners = {s: [] for s in scenarios}

    for w_tlp in w_tlp_values:
        remaining = 1.0 - w_tlp
        w_other = remaining / 4.0
        weights = {
            "TLP": w_tlp,
            "TAR": w_other,
            "LSC": w_other,
            "SE": w_other,
            "TP": w_other,
        }

        scores = compute_composite_scores(normalized, weights)
        all_scores[w_tlp] = scores

        for scenario in scenarios:
            scenario_scores = [
                (p, scores.get((p, scenario), float("inf")))
                for p in protocols
            ]
            best = min(scenario_scores, key=lambda x: x[1])
            winners[scenario].append(best[0])

    return {
        "w_tlp_values": w_tlp_values,
        "winners": winners,
        "scores": all_scores,
    }


def monte_carlo_robustness(
    normalized: dict[tuple[str, str], dict[str, float]],
    scenarios: list[str],
    protocols: list[str],
    n_samples: int = 10_000,
    rng_seed: int = 42,
) -> dict[str, dict[str, float]]:
    """
    Sample weight vectors from symmetric Dirichlet(1,1,1,1,1).
    For each vector and scenario, determine the winning protocol.

    Returns
    -------
    {protocol: {scenario: win_fraction}}
    """
    rng = np.random.RandomState(rng_seed)

    # Initialize counters
    win_counts: dict[str, dict[str, int]] = {
        p: {s: 0 for s in scenarios} for p in protocols
    }

    # Sample Dirichlet vectors
    alpha = np.ones(len(METRIC_NAMES))
    weight_samples = rng.dirichlet(alpha, size=n_samples)

    for sample in weight_samples:
        weights = dict(zip(METRIC_NAMES, sample))
        scores = compute_composite_scores(normalized, weights)

        for scenario in scenarios:
            scenario_scores = [
                (p, scores.get((p, scenario), float("inf")))
                for p in protocols
            ]
            best = min(scenario_scores, key=lambda x: x[1])
            win_counts[best[0]][scenario] += 1

    # Convert to fractions
    win_fractions = {
        p: {s: count / n_samples for s, count in scenarios_counts.items()}
        for p, scenarios_counts in win_counts.items()
    }

    return win_fractions
