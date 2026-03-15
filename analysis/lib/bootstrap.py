"""
BCa bootstrap confidence intervals (Section IV-C.3).

Implements bias-corrected and accelerated bootstrap for distribution-free,
skew-robust confidence intervals on run-level summary statistics.

Input: array of R=30 run-level summary values (e.g., 30 per-run p99 values).
Output: (point_estimate, ci_low, ci_high) at 95% confidence.

BCa is chosen over percentile bootstrap because latency distributions are
right-skewed and heavy-tailed (Section IV-C.3). The acceleration factor
corrects for skewness in the bootstrap distribution via jackknife.
"""

import numpy as np
from scipy import stats as sp_stats


def bca_bootstrap_ci(
    data: np.ndarray,
    n_resamples: int = 10_000,
    confidence: float = 0.95,
    stat_fn=np.median,
    rng_seed: int = 0,
) -> tuple[float, float, float]:
    """
    Bias-corrected and accelerated (BCa) bootstrap confidence interval.

    Parameters
    ----------
    data : 1D array of run-level summary statistics (typically length 30).
    n_resamples : number of bootstrap resamples (10,000 per Section IV-C.3).
    confidence : confidence level (0.95 for 95% CI).
    stat_fn : statistic to compute (default: median).
    rng_seed : random seed for reproducibility.

    Returns
    -------
    (point_estimate, ci_low, ci_high)
    """
    data = np.asarray(data, dtype=np.float64)
    n = len(data)

    if n == 0:
        return (np.nan, np.nan, np.nan)

    theta_hat = float(stat_fn(data))

    # Degenerate case: all values identical
    if np.all(data == data[0]):
        return (theta_hat, theta_hat, theta_hat)

    rng = np.random.RandomState(rng_seed)
    alpha = 1.0 - confidence

    # Bootstrap distribution
    boot_indices = rng.randint(0, n, size=(n_resamples, n))
    boot_stats = np.array([stat_fn(data[idx]) for idx in boot_indices])

    # Bias correction factor z0
    prop_less = np.mean(boot_stats < theta_hat)
    # Clamp to avoid infinite z0
    prop_less = np.clip(prop_less, 1e-10, 1.0 - 1e-10)
    z0 = sp_stats.norm.ppf(prop_less)

    # Acceleration factor (jackknife)
    jackknife_stats = np.empty(n)
    for i in range(n):
        jack_sample = np.delete(data, i)
        jackknife_stats[i] = stat_fn(jack_sample)

    jack_mean = np.mean(jackknife_stats)
    diffs = jack_mean - jackknife_stats
    numerator = np.sum(diffs**3)
    denominator = 6.0 * (np.sum(diffs**2) ** 1.5)

    if abs(denominator) < 1e-15:
        a = 0.0
    else:
        a = numerator / denominator

    # Adjusted percentiles
    z_alpha_low = sp_stats.norm.ppf(alpha / 2)
    z_alpha_high = sp_stats.norm.ppf(1 - alpha / 2)

    def _adjusted_quantile(z_alpha):
        numer = z0 + z_alpha
        denom = 1.0 - a * numer
        if abs(denom) < 1e-15:
            return 0.5
        adjusted_z = z0 + numer / denom
        return sp_stats.norm.cdf(adjusted_z)

    alpha1 = _adjusted_quantile(z_alpha_low)
    alpha2 = _adjusted_quantile(z_alpha_high)

    # Clamp to valid percentile range
    alpha1 = np.clip(alpha1, 0.0, 1.0)
    alpha2 = np.clip(alpha2, 0.0, 1.0)

    ci_low = float(np.percentile(boot_stats, 100 * alpha1))
    ci_high = float(np.percentile(boot_stats, 100 * alpha2))

    return (theta_hat, ci_low, ci_high)
