#!/usr/bin/env python3
"""The power calculation of R-661: what n can detect, for KS distance, outcome fractions and lifetimes.

Usage: python3 -I power.py     (fixed seed; the Monte Carlo takes under a minute)

Two-sample KS: critical distance D_c = c(a) sqrt(2/n), c(a) = sqrt(-ln(a/2)/2), n per sample. Fractions: the normal
approximation for two independent proportions, n = (z_a/2 + z_b)^2 2p(1-p) / delta^2. Paired fractions (McNemar):
n = (z_a/2 + z_b)^2 d / delta^2, d the discordant fraction. Lifetimes (log-rank): total events = 4 (z_a/2 + z_b)^2 /
ln(HR)^2. Alpha 0.05 throughout, power 0.8 where a power is named.
"""
from math import comb, erf, exp, log, sqrt

import numpy as np

rng = np.random.default_rng(661)
Z = 1.959964 + 0.841621


def c(a):
    return sqrt(-0.5 * log(a / 2))


def ks(a, b):
    x = np.argsort(np.concatenate([a, b]), kind="mergesort")
    return np.abs(np.cumsum(np.where(x < len(a), 1 / len(a), -1 / len(b)))).max()


def shift_for(d):
    lo, hi = 0.0, 5.0
    for _ in range(60):
        mid = (lo + hi) / 2
        lo, hi = (mid, hi) if erf(mid / 2 / sqrt(2)) < d else (lo, mid)
    return lo


def power(n, d, alt, reps):
    crit, hit = c(0.05) * sqrt(2 / n), 0
    mu = shift_for(d)
    for _ in range(reps):
        if alt == "normal shift":
            a, b = rng.standard_normal(n), rng.standard_normal(n) + mu
        elif alt == "tail mass":  # a fraction d of the mass moved beyond the other sample's range
            a, b = rng.random(n), rng.random(n)
            b = np.where(rng.random(n) < d, b + 1.0, b)
        else:  # "at the median": mass d moved across the median over a width d, so the distance is d only there
            a, b = rng.random(n), rng.random(n)
            b = np.where((b > 0.5 - d) & (b < 0.5), b + d, b)
        hit += ks(a, b) > crit
    return hit / reps


print(f"c(0.05) = {c(0.05):.4f}; c(0.05/9) = {c(0.05 / 9):.4f}")
print("n: D_c at 5%; with Bonferroni over 9 tests")
for n in (256, 1000, 4096, 6257, 7366, 10000):
    print(f"  {n}: {c(0.05) * sqrt(2 / n):.4f}; {c(0.05 / 9) * sqrt(2 / n):.4f}")
for d in (0.02, 0.03, 0.05):
    print(f"n with D_c = {d}: {2 * (c(0.05) / d) ** 2:.0f}; with Bonferroni over 9: {2 * (c(0.05 / 9) / d) ** 2:.0f}")
print("Monte Carlo power of the 5% test when the true distance is D_c")
for n, reps in ((256, 800), (10000, 300)):
    for alt in ("normal shift", "tail mass", "at the median"):
        print(f"  n = {n}, {alt}: {power(n, c(0.05) * sqrt(2 / n), alt, reps):.2f} ({reps} replicates)")
for d in (0.022, 0.025, 0.028):
    print(f"  n = 10000, at the median, true distance {d}: {power(10000, d, 'at the median', 300):.2f} (300 replicates)")
print("n: smallest difference in a fraction p detected at 80% power, independent samples; p = 0.5, 1/3, 0.1")
for n in (256, 1000, 4096, 6257, 7366, 10000):
    print(f"  {n}: " + ", ".join(f"{Z * sqrt(2 * p * (1 - p) / n):.3f}" for p in (0.5, 1 / 3, 0.1)))
print("delta: n at 80% power; p = 0.5, 1/3, 0.1; paired with d = 0.33")
for dl in (0.01, 0.02, 0.03, 0.05):
    print(f"  {dl}: " + ", ".join(f"{Z ** 2 * 2 * p * (1 - p) / dl ** 2:.0f}" for p in (0.5, 1 / 3, 0.1))
          + f"; {Z ** 2 * 0.33 / dl ** 2:.0f}")
print("Log-rank at 80% power: events in both arms, the hazard ratio detected")
for ev, name in ((2 * 6257, "cold equal, 10^4 a side, T = 200"), (2 * 7366, "cold 1:2:4, 10^4 a side, T = 200"),
                 (2 * 128, "256 a side, half censored")):
    print(f"  {name}: {ev} events, HR {exp(2 * Z / sqrt(ev)):.3f}")
print("The median of n values: 95% half-width in units of the spread s (normal), 1.96 * 1.2533 / sqrt(n)")
for n in (256, 4096, 10000):
    print(f"  {n}: {1.96 * 1.2533 / sqrt(n):.3f} s")
print("The median, distribution-free: the narrowest rank interval symmetric about the middle with exact coverage >= 95%")
for n in (256, 4096, 10000):
    lo = n // 2
    while True:
        cover = sum(comb(n, k) for k in range(lo, n - lo + 1)) / 2**n
        if cover >= 0.95:
            break
        lo -= 1
    print(f"  {n}: ranks {lo} to {n - lo + 1}, quantiles {lo / n:.3f} to {(n - lo + 1) / n:.3f}, coverage {cover:.4f}")
print("n = 256, c bounds of one direction: rank 145's end moves out by c ranks among the 256 - c measured ratios")
for c_ in (111, 112, 127):
    print(f"  c = {c_}: {256 - c_} measured; {'unbounded' if 256 - c_ < 145 else 'bounded'}")
