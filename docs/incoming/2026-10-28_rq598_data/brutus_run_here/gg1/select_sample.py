#!/usr/bin/env python3
"""Pick the GG-1 pilot sample: 16 of the 1212 points, stratified on the paper's own Fig. 1 (eps = 1e-6) lifetimes, and
spread across the map within each stratum. Fig. 1 is used ONLY to choose which points to time; nothing here is a result.
Deterministic: no random numbers.

usage: select_sample.py <gg1_fig1_left_eps1e-6.csv> <out sample.csv>
"""
import csv
import math
import sys

rows = list(csv.DictReader(open(sys.argv[1])))
pts = []
for r in rows:
    black = r["log10_T"].startswith("censored")
    pts.append(dict(i=int(r["i"]), j=int(r["j"]), x=float(r["x"]), y=float(r["y"]),
                    lg=None if black else float(r["log10_T"])))
# strata on the Fig. 1 log10 T, and how many to take from each
strata = [("short", lambda p: p["lg"] is not None and p["lg"] < 1.0, 2),
          ("log T 1.0-1.5", lambda p: p["lg"] is not None and 1.0 <= p["lg"] < 1.5, 3),
          ("log T 1.5-2.0", lambda p: p["lg"] is not None and 1.5 <= p["lg"] < 2.0, 3),
          ("log T 2.0-2.5", lambda p: p["lg"] is not None and 2.0 <= p["lg"] < 2.5, 3),
          ("log T 2.5-3.0", lambda p: p["lg"] is not None and 2.5 <= p["lg"], 2),
          ("black, T>1000", lambda p: p["lg"] is None, 3)]
chosen = []
for name, f, n in strata:
    pool = [p for p in pts if f(p)]
    # seed: the pool point nearest the pool's median lifetime (or the first black one), then farthest-point on (x, y)
    pool.sort(key=lambda p: (p["i"], p["j"]))
    cand = []
    for _ in range(n):
        if not chosen and not cand:
            pick = pool[len(pool) // 2]
        else:
            ref = chosen + cand
            pick = max(pool, key=lambda p: (min(math.hypot(p["x"] - q["x"], p["y"] - q["y"]) for q in ref), -p["i"], -p["j"]))
        pick = dict(pick, stratum=name)
        cand.append(pick)
    chosen += cand
with open(sys.argv[2], "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["i", "j", "x", "y", "stratum", "fig1_log10_T_for_selection_only"])
    for p in chosen:
        w.writerow([p["i"], p["j"], p["x"], p["y"], p["stratum"], "censored>1000" if p["lg"] is None else p["lg"]])
for p in chosen:
    print(p["i"], p["j"], p["stratum"], p["lg"])
