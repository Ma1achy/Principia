#!/usr/bin/env python3
"""BR-1 ensemble cost at the authors' (eps, L_w) pairs, ESTIMATED from our measurements at eps >= 1e-6 (nothing here was
run below 1e-6). Model: wall per run = steps x ms/step.
 - ms/step is fitted as linear in -log10(eps) on the 9 full-length runs at eps = 1e-3 .. 1e-6, L_w = 128 (measured),
   then extrapolated 4-5 decades beyond the fit: this is the weak step.
 - steps: measured median at 1e-6 (2653, set by the 2^-8 output cadence, floor 2561); at tighter eps the code's own
   step law  log10 eta = -0.012 |log10 eps| - 0.40  (Brutus.cpp get_eta, eps > 1e-50) shrinks steps by up to 10^(-0.012 d).
 - L_w = 88 vs 128: measured equal cost (both fit in two 64-bit limbs).
usage: projections.py <br1 results csv> <br1 truncated-to-t10 results csv>
"""
import csv
import statistics as st
import sys

import numpy as np

rows = list(csv.DictReader(open(sys.argv[1])))
tr = list(csv.DictReader(open(sys.argv[2])))
full = [r for r in rows if r["stratum"].startswith("full") and r["Lw"] == "128" and r["status"] == "ok"]
xs, ys = [], []
for e in ("1e-3", "1e-4", "1e-5", "1e-6"):
    v = [1e3 * float(r["wall_s"]) / int(r["steps"]) for r in full if r["eps"] == e]
    xs.append(-float(np.log10(float(e))))
    ys.append(st.median(v))
    print(f"eps={e}: median ms/step {ys[-1]:.3f} over {len(v)} runs (MEASURED)")
slope, icpt = np.polyfit(xs, ys, 1)
print(f"fit ms/step = {icpt:.3f} + {slope:.4f} x (-log10 eps)   [fit range 3..6 decades]")
steps6 = st.median(int(r["steps"]) for r in full if r["eps"] == "1e-6")
wall6 = st.median(float(r["wall_s"]) for r in full if r["eps"] == "1e-6")
allw = [float(r["wall_s"]) for r in full if r["eps"] == "1e-6"] + [float(r["wall_s"]) for r in tr if r["eps"] == "1e-6"]
tail = st.mean(allw) / st.median(allw)
print(f"eps=1e-6 L_w=128 wall to t=10 (MEASURED): median of 9 full runs {wall6:.2f} s; all 12 runs to t=10 "
      f"(9 full + 3 authors-truncated, run out to 10): median {st.median(allw):.2f}, mean {st.mean(allw):.2f}, max {max(allw):.2f}; mean/median = {tail:.2f}")
for eps_s, lw in (("1e-11", 128), ("1e-10", 88)):
    d = -float(np.log10(float(eps_s)))
    msps = icpt + slope * d
    lo = steps6 * 1.0 * msps / 1e3
    hi = steps6 * (1 + 0.15) * msps / 1e3
    print(f"\nESTIMATE at eps={eps_s}, L_w={lw}: ms/step {msps:.2f} (linear extrapolation); steps {steps6:.0f}-{steps6 * 1.15:.0f}; "
          f"wall per run {lo:.1f}-{hi:.1f} s (median-like run)")
    for N in (1000, 10000):
        for lab, k in (("one pair", 1), ("both pairs, as the authors' convergence test needs", 2)):
            lo_h = N * lo * tail * k / 3600
            hi_h = N * hi * tail * k / 3600
            print(f"   N={N:6d}, {lab:50s}: {lo_h:6.1f}-{hi_h:6.1f} core-hours (with the mean/median tail factor {tail:.2f} from our 12 runs)")
print("\nNOT captured: runs that need many more steps than the 2561-output floor (the authors' truncated files stop early, at their own"
      " failure points; the pilot counted 44 of 180, a count not verified from committed material);"
      " a 12-run sample cannot show that tail. Treat the figures as the cost of the typical run.")
