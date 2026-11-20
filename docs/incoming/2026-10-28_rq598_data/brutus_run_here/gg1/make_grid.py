#!/usr/bin/env python3
"""GG-1 grid on the Agekyan-Anosova map at spacing 1/64, cross-checked against the Fig. 1 extraction.

Region (Boekholt, Portegies Zwart & Valtonen 2020, Appendix A + the repo's Agekyan-Anosova frame): bodies 1 and 2
at (-1/2, 0) and (1/2, 0), body 3 at (x, y) = (i/64, j/64), with 0 < x < 1/2, y > 0, (x + 1/2)^2 + y^2 < 1.
The 1/64 grid points are exact in binary, so the tests below are done in integers.

usage: make_grid.py <gg1_fig1_left_eps1e-6.csv from data/rq598-gg1-fig1-v2> <out grid.csv>
"""
import csv
import sys

pts = []
for i in range(1, 64):
    for j in range(1, 129):
        # 0 < i/64 < 1/2  <=>  0 < i < 32;  (x + 1/2)^2 + y^2 < 1  <=>  (i + 32)^2 + j^2 < 64^2
        if i < 32 and (i + 32) ** 2 + j * j < 64 * 64:
            pts.append((i, j))
n = len(pts)
print(f"grid points: {n}   i = {min(p[0] for p in pts)}..{max(p[0] for p in pts)}   j = {min(p[1] for p in pts)}..{max(p[1] for p in pts)}")
if n != 1212:
    sys.exit(f"STOP: expected 1212 points, got {n}")
with open(sys.argv[1]) as f:
    fig = {(int(r["i"]), int(r["j"])) for r in csv.DictReader(f)}
print(f"Fig. 1 extraction (left panel): {len(fig)} points; identical sets: {fig == set(pts)}")
if fig != set(pts):
    sys.exit("STOP: grid differs from the Fig. 1 extraction")
with open(sys.argv[2], "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["grid_index", "i", "j", "x", "y"])
    for k, (i, j) in enumerate(pts, 1):
        w.writerow([k, i, j, i / 64, j / 64])
print("wrote", sys.argv[2])
