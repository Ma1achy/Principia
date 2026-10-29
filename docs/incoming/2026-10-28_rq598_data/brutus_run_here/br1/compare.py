#!/usr/bin/env python3
"""Compare our trajectories ("Brutus, run here") to the authors' (pgbreen/NVM @ a35c378), point by point.

Both files: columns x1 y1 x2 y2 at t = k/256; body 3 = -(x1 + x2). Compared over the rows both have.
Per run and per (eps, Lw): the largest position difference over any body at any time, the first time the difference
exceeds 1e-3 / 1e-2 / 0.1, and the position-only phase distance of Breen et al. 2020 Appendix A (A1). The paper's
delta^2 = (1/12) sum over 3 particles and 4 coordinates (x, y, vx, vy); the authors' files hold positions only, so we
use the 6 position terms: delta_pos^2 = (1/6) sum over 3 particles and 2 coordinates of (dq)^2, averaged over the
rows. "delta_pos" below is its square root (RMS); mse is delta_pos^2. Their criterion: average delta < 0.1.

usage: compare.py <nvm local dir> <sample_12.csv> <traj dir> <out.csv>
"""
import csv
import os
import sys

import numpy as np

root, sample, trajdir, out = sys.argv[1:5]
configs = [("1e-6", "128"), ("1e-5", "128"), ("1e-4", "128"), ("1e-6", "88")]


def load(path):
    a = np.loadtxt(path, ndmin=2)
    return np.hstack([a, -(a[:, 0:2] + a[:, 2:4])])  # x1 x2 x3 as 6 columns


def first_exceed(d, thr):
    k = np.nonzero(d > thr)[0]
    return "" if len(k) == 0 else f"{k[0] / 256:.6g}"


rows = []
for r in csv.DictReader(open(sample)):
    theirs = load(os.path.join(root, r["set"] + "_data", r["name"] + ".dat"))
    for eps, lw in configs:
        f = os.path.join(trajdir, f"{r['name']}_eps{eps}_Lw{lw}.dat")
        if not os.path.exists(f):
            continue
        ours = load(f)
        n = min(len(ours), len(theirs))
        dq = ours[:n] - theirs[:n]
        per_body = np.sqrt(dq[:, 0::2] ** 2 + dq[:, 1::2] ** 2)  # |dx| of each of 3 bodies, per row
        dmax = per_body.max(axis=1)
        mse = (dq ** 2).sum(axis=1) / 6.0
        rows.append([r["name"], r["set"], r["stratum"], eps, lw, len(theirs), len(ours), n,
                     f"{dmax[0]:.3g}", f"{dmax.max():.3g}", f"{dmax[: min(n, 256)].max():.3g}",
                     first_exceed(dmax, 1e-3), first_exceed(dmax, 1e-2), first_exceed(dmax, 0.1),
                     f"{np.sqrt(mse.mean()):.4g}", f"{mse.mean():.4g}", "yes" if np.sqrt(mse.mean()) < 0.1 else "no"])
with open(out, "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["run", "set", "stratum", "eps", "Lw", "rows_authors", "rows_ours", "rows_compared", "max_diff_at_t0",
                "max_diff_any_t", "max_diff_t_le_1", "first_t_diff_gt_1e-3", "first_t_diff_gt_1e-2",
                "first_t_diff_gt_0.1", "delta_pos_rms_mean", "mse_mean", "mean_delta_lt_0.1"])
    w.writerows(rows)
for x in rows:
    print(" ".join(str(v) for v in [x[0], x[3], x[4], x[7]] + x[8:]))
