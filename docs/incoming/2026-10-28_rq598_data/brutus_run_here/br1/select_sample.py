#!/usr/bin/env python3
"""Pick the BR-1 pilot sample from the authors' own runs (pgbreen/NVM @ a35c378, trainex/data/run10-99.dat and
validex/data/run9910-9999.dat; copies live in docs/incoming/_local/, never committed).

Each file: columns x1 y1 x2 y2 (positions at t = k/256); body 3 = -(x1 + x2) (barycentre at 0). Most files have the full
2561 rows (t = 0..10); some are SHORTER (the authors' converged solution stops early, e.g. run96 has 5 rows). Sample:
9 full-length runs, 3 each from the tertiles of the MINIMUM PAIR SEPARATION along the authors' stored trajectory (a
proxy for how hard the run is for Brutus), plus 3 truncated runs (shortest-but-one, middle, longest truncated) which
we run to the authors' last row. Spread over trainex and validex by position in the sorted order. Deterministic.

The sample file records run names, strata and row counts only. No coordinate, and no value computed from the authors'
files alone, is written: run_br1.py reads each initial condition from the local copy at run time.

usage: select_sample.py <dir holding trainex_data/ and validex_data/> <out sample.csv>
"""
import csv
import glob
import math
import os
import sys

import numpy as np

root = sys.argv[1]
runs = []
for sub in ("trainex_data", "validex_data"):
    for f in sorted(glob.glob(os.path.join(root, sub, "run*.dat"))):
        a = np.loadtxt(f)
        assert a.ndim == 2 and a.shape[1] == 4 and a.shape[0] >= 1, (f, a.shape)
        x1, x2 = a[:, 0:2], a[:, 2:4]
        x3 = -(x1 + x2)
        d = np.minimum.reduce([np.hypot(*(x1 - x2).T), np.hypot(*(x1 - x3).T), np.hypot(*(x2 - x3).T)])
        with open(f) as fh:
            first = fh.readline().split()
        runs.append(dict(name=os.path.basename(f)[:-4], set=sub[:-5], x2x=first[2], x2y=first[3],
                         x1x=first[0], x1y=first[1], nrows=a.shape[0], rmin=float(d.min()), tmin=float(d.argmin()) / 256,
                         sing=math.hypot(float(first[2]) + 0.5, float(first[3]))))
full = sorted((r for r in runs if r["nrows"] == 2561), key=lambda r: (r["rmin"], r["name"]))
short = sorted((r for r in runs if r["nrows"] < 2561), key=lambda r: (r["nrows"], r["name"]))
print("runs found:", len(runs), " full length:", len(full), " truncated:", len(short))
chosen = []
for q in range(3):
    block = full[q * len(full) // 3:(q + 1) * len(full) // 3]
    for pos in (0.15, 0.5, 0.85):
        chosen.append(dict(block[int(pos * (len(block) - 1))], stratum=f"full, min-sep tertile {q + 1} (1 = closest)"))
for pos in (0.15, 0.5, 0.85):
    chosen.append(dict(short[int(pos * (len(short) - 1))], stratum="truncated by the authors"))
with open(sys.argv[2], "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["name", "set", "stratum", "authors_rows", "t_end_to_run"])
    for r in chosen:
        w.writerow([r["name"], r["set"], r["stratum"], r["nrows"], (r["nrows"] - 1) / 256])
for r in chosen:
    print(r["name"], r["set"], r["stratum"], r["nrows"])
