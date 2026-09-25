"""
Robust exponent estimation.

§7.10 showed the refinement CRITERION works but the ESTIMATOR is fragile: three jitter samples
gave alpha = 6.691 for total energy, whose true exponent is exactly 1.0 (its spread is
|grad E| * delta, so it must be linear). Every prediction in that quad failed with it.

Production has only TWO scales (a parent quad and its children), fewer than the three used
there, so this must be characterised before it is relied on.

GROUND TRUTH. Total energy is the ideal test field: its exponent is known analytically to be
1.0, in every region, at every playhead. Any estimator that cannot return 1.0 for energy is
broken, and the amount by which it misses is a direct error measure -- no reference run needed.

ESTIMATORS
  ols2        two-point log-log slope (what production can actually do)
  ols3        three-point least squares (what section 7.10 used)
  ols5        five-point least squares (accuracy ceiling)
  theil       Theil-Sen median-of-pairwise-slopes: robust to one bad point
  ratio_med   median of adjacent log-ratios
  clamped     ols2 clamped to the physically meaningful range [0, 1]
"""
import numpy as np


def _fit_ols(x, y):
    ok = np.isfinite(x) & np.isfinite(y)
    if ok.sum() < 2:
        return np.nan
    return float(np.polyfit(x[ok], y[ok], 1)[0])


def est_ols(deltas, spreads):
    d = np.asarray(deltas, float); s = np.asarray(spreads, float)
    ok = (s > 1e-14) & np.isfinite(s)
    if ok.sum() < 2:
        return np.nan
    return _fit_ols(np.log(d[ok]), np.log(s[ok]))


def est_theil(deltas, spreads):
    """Median of all pairwise slopes — one bad sample cannot dominate."""
    d = np.asarray(deltas, float); s = np.asarray(spreads, float)
    ok = (s > 1e-14) & np.isfinite(s)
    d, s = d[ok], s[ok]
    if len(d) < 2:
        return np.nan
    sl = [(np.log(s[j]) - np.log(s[i])) / (np.log(d[j]) - np.log(d[i]))
          for i in range(len(d)) for j in range(i + 1, len(d))
          if abs(np.log(d[j]) - np.log(d[i])) > 1e-12]
    return float(np.median(sl)) if sl else np.nan


def est_ratio_median(deltas, spreads):
    d = np.asarray(deltas, float); s = np.asarray(spreads, float)
    o = np.argsort(d); d, s = d[o], s[o]
    ok = (s > 1e-14) & np.isfinite(s)
    d, s = d[ok], s[ok]
    if len(d) < 2:
        return np.nan
    sl = [(np.log(s[i + 1]) - np.log(s[i])) / (np.log(d[i + 1]) - np.log(d[i]))
          for i in range(len(d) - 1)]
    return float(np.median(sl))


def est_clamped(deltas, spreads, lo=0.0, hi=1.0):
    a = est_ols(deltas, spreads)
    return np.nan if not np.isfinite(a) else float(np.clip(a, lo, hi))


ESTIMATORS = {
    'ols2':      lambda d, s: est_ols(d[-2:], s[-2:]),
    'ols3':      lambda d, s: est_ols(d[-3:], s[-3:]),
    'ols5':      est_ols,
    'theil':     est_theil,
    'ratio_med': est_ratio_median,
    'clamped2':  lambda d, s: est_clamped(d[-2:], s[-2:]),
}
