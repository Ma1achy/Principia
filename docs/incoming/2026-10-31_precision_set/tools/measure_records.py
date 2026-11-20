#!/usr/bin/env python3
"""Measure BPZ 2015's additional files (R-661): planarity, placeholder rows, end times and the horizon in Principia's units.

Usage: python3 -I measure_records.py ../data/40668_2014_5_MOESM*_ESM.dat

Columns: index; m x y z vx vy vz for each body at the start, likewise at the end; t e Lw. The out-of-plane extent is
the distance of every final position from the plane through the three bodies at t = 0, and the final velocity along
that plane's normal. The horizon: the integrator's units are M = G = I = 1 (integrator contract Part 5), reached from
BPZ's M = G = 1 by a length I^(1/2), so a time t_int is t_int * I^(3/4) in Hénon units, I the start's moment of inertia
about its centre of mass.
"""
import sys

import numpy as np

CAP = 200  # R-610


def main(paths):
    for path in paths:
        d = np.array([ln.split() for ln in open(path) if not ln.startswith("#")], float)
        m = d[:, [1, 8, 15]]
        r0 = np.stack([d[:, 2:5], d[:, 9:12], d[:, 16:19]], 1)
        v0 = np.stack([d[:, 5:8], d[:, 12:15], d[:, 19:22]], 1)
        r1 = np.stack([d[:, 23:26], d[:, 30:33], d[:, 37:40]], 1)
        v1 = np.stack([d[:, 26:29], d[:, 33:36], d[:, 40:43]], 1)
        t, e, lw = d[:, 43], d[:, 44], d[:, 45]
        kept = ~np.all(np.concatenate([r1, v1], 1).reshape(len(d), -1) == 0, axis=1)
        n = np.cross(r0[:, 1] - r0[:, 0], r0[:, 2] - r0[:, 0])
        n /= np.linalg.norm(n, axis=1, keepdims=True)
        h = np.abs(((r1 - r0[:, [0], :]) * n[:, None, :]).sum(2))[kept].max(1)
        hv = np.abs((v1 * n[:, None, :]).sum(2))[kept].max(1)
        com = (m[:, None, :] @ r0)[:, 0] / m.sum(1, keepdims=True)
        vcm = (m[:, None, :] @ v0)[:, 0] / m.sum(1, keepdims=True)
        rc, vc = r0 - com[:, None, :], v0 - vcm[:, None, :]
        l0 = np.linalg.norm((m[:, :, None] * np.cross(rc, vc)).sum(1), axis=1)
        inertia = (m[:, :, None] * rc**2).sum((1, 2))
        t_cap = CAP * inertia**0.75
        print(path.rsplit("/", 1)[-1])
        print(f"  rows {len(d)}; masses {np.round(m[0], 6).tolist()}; max start speed {np.abs(v0).max():.3g}; "
              f"|L0| median {np.median(l0):.3g} max {l0.max():.3g}")
        print(f"  all-zero finals {(~kept).sum()}; of them t = 0: {(t[~kept] == 0).sum()}, e = 0: "
              f"{(e[~kept] == 0).sum()}, Lw = 0: {(lw[~kept] == 0).sum()}")
        print(f"  out of plane, position: median {np.median(h):.3g}, max {h.max():.3g}; velocity: median "
              f"{np.median(hv):.3g}, max {hv.max():.3g}")
        print(f"  stored t = 501 (last state kept): {(t == 501).sum()}; undissolved at 501 with placeholders counted: "
              f"{100 * ((t == 501).sum() + (~kept).sum()) / len(d):.1f}%")
        print(f"  rows whose t is not a whole number: {(t != np.floor(t)).sum()}")
        print(f"  ended at t <= 200 Henon: {(kept & (t <= 200)).sum()}; below 501: {(kept & (t < 501)).sum()}")
        print(f"  Henon time of integrator T = {CAP}, min/median/max: {t_cap.min():.1f} / {np.median(t_cap):.1f} / "
              f"{t_cap.max():.1f}; ended within it: {(kept & (t < 501) & (t <= t_cap)).sum()}")


if __name__ == "__main__":
    main(sys.argv[1:])
