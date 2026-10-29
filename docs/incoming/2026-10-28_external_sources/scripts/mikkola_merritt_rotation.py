"""Mikkola & Merritt 2008 (AJ 135, 2398), Table 1, first three bodies: our planar rotation, worked and checked.

Usage: python3 -I mikkola_merritt_rotation.py <table1_N3_reconciled.csv> <out.csv> [chatgpt.json]
Working. Every row of Table 1 has Z = X and all velocities zero. So every body lies in the plane X - Z = 0, whose
in-plane orthonormal basis is e1 = (1, 0, 1)/sqrt2, e2 = (0, 1, 0), with normal n = (1, 0, -1)/sqrt2. A body at
(X, Y, X) has components x = r.e1 = 2X/sqrt2 = sqrt2 X, y = r.e2 = Y, and r.n = 0. The bodies start at rest, so every
force (along the separations, which lie in the plane) stays in the plane, the velocities stay in it, and the motion is
planar for all time: the planar problem with (x, y) = (sqrt2 X, Y), (vx, vy) = (0, 0) is the same problem. The map is
orthonormal: completed by e1 x e2 = (-1, 0, 1)/sqrt2 = -n as the third axis it is a rotation (det +1; with n itself
as the third axis the determinant is -1), so every distance, the energy and |L| are unchanged; L = 0.
Checks printed: r.n = 0 for each body, the pairwise distances before and after, the energy before and after (G = 1).
Optionally compares with ChatGPT's planar_xy.
"""
import csv
import json
import sys

import mpmath as mp

mp.mp.dps = 50
rows = list(csv.DictReader(open(sys.argv[1], encoding="utf-8")))
s2 = mp.sqrt(2)
r3, r2, m = [], [], []
for r in rows:
    X, Y, Z = (mp.mpf(r[k]) for k in ("X", "Y", "Z"))
    r3.append((X, Y, Z))
    m.append(mp.mpf(r["m"]))
    print(f"body {r['body']}: Z - X = {Z - X}, r.n = {(X - Z) / s2}")
    r2.append((s2 * X, Y))


def dist(p, q):
    return mp.sqrt(sum((a - b) ** 2 for a, b in zip(p, q)))


worst = 0
U3 = U2 = 0
for i in range(3):
    for j in range(i + 1, 3):
        d3, d2 = dist(r3[i], r3[j]), dist(r2[i], r2[j])
        worst = max(worst, abs(d3 - d2))
        U3 += m[i] * m[j] / d3
        U2 += m[i] * m[j] / d2
print(f"largest change in a pairwise distance: {mp.nstr(worst, 3)}; E (all at rest) 3D = {mp.nstr(-U3, 25)}, "
      f"planar = {mp.nstr(-U2, 25)}")
with open(sys.argv[2], "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["body", "m", "x", "y", "vx", "vy"])
    for r, (x, y) in zip(rows, r2):
        w.writerow([r["body"], r["m"], mp.nstr(x, 30), mp.nstr(y, 30), 0, 0])
if len(sys.argv) > 3:
    cg = json.load(open(sys.argv[3], encoding="utf-8"))["initial_conditions"]
    dmax = max(abs(mp.mpf(c["planar_xy"][k]) - p[k]) for c, p in zip(cg, r2) for k in range(2))
    print(f"largest |ours - ChatGPT| in planar (x, y): {mp.nstr(dmax, 3)} (ChatGPT's are binary64 values)")
