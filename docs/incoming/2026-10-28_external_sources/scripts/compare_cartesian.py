"""Compares ChatGPT's Cartesian conversion of Janković et al. 2020 (reading A's JSON) with ours (cartesian_ours.csv).

Usage: python3 -I compare_cartesian.py <jankovic_2020_catalogue_99.json> <cartesian_ours.csv>
ChatGPT's state order is x1,y1,x2,y2,x3,y3,vx1,vy1,vx2,vy2,vx3,vy3; ours lists the non-zero components x1, x2, x3,
vy1, vy2, vy3 (every y and vx is zero by construction). Prints the largest absolute difference over all 99 rows and
whether every y and vx in ChatGPT's states is exactly zero.
"""
import csv
import json
import sys

import mpmath as mp

mp.mp.dps = 50
d = json.load(open(sys.argv[1], encoding="utf-8"))
ours = {r["nr"]: r for r in csv.DictReader(open(sys.argv[2], encoding="utf-8"))}
worst, where, zero_ok = mp.mpf(0), None, True
for o in d["orbits"]:
    s = [mp.mpf(x) for x in o["cartesian_nominal"]]
    r = ours[str(o["paper_row"])]
    pairs = [(s[0], r["x1"]), (s[2], r["x2"]), (s[4], r["x3"]), (s[7], r["vy1"]), (s[9], r["vy2"]), (s[11], r["vy3"])]
    for k, (a, b) in enumerate(pairs):
        diff = abs(a - mp.mpf(b))
        if diff > worst:
            worst, where = diff, (o["paper_row"], ["x1", "x2", "x3", "vy1", "vy2", "vy3"][k])
    if any(s[i] != 0 for i in (1, 3, 5, 6, 8, 10)):
        zero_ok = False
print(f"largest |ChatGPT - ours| over 99 rows x 6 components: {mp.nstr(worst, 3)} (row {where[0]}, {where[1]}); "
      f"ours printed to 25 significant digits")
print(f"every y and vx in ChatGPT's states is zero: {zero_ok}")
