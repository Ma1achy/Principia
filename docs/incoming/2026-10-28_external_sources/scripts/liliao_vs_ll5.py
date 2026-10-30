#!/usr/bin/env python3
"""Compare Li & Liao 2018's Tables A1-A5 (our reconciled reading) with the authors' own 316-orbit free-fall table.

Usage: python3 -I liliao_vs_ll5.py <tablesA1toA5_reconciled.csv> <free-fall-3b-pictures.md> <out.csv>

The second file is docs/incoming/2026-10-27_hp_ics_freefall/data/liliao_github/free-fall-3b-pictures.md (LL-5's
source, committed by R-631). Both use r1 = (-0.5, 0), r2 = (0.5, 0), r3 = (x, y), at rest, G = 1.
Matching rule: same masses (m1, m2, m3) and the same x and y as decimal strings; T is then compared as a number.
Labels F_i are not used for matching (the two lists number orbits independently).
"""
import csv
import re
import sys
from decimal import Decimal

ROW = re.compile(r"<th>F<sub>(\d+)</sub>\(([^)]*)\)</th>\s*<th>\s*([-\d.]+)</th>\s*<th>\s*([-\d.]+)</th>\s*"
                 r"<th>\s*([-\d.]+)</th>", re.S)


def norm(s):
    return str(Decimal(s).normalize())


def main(paper_csv, md, out):
    gh = {}
    for fi, masses, x, y, t in ROW.findall(open(md, encoding="utf-8").read()):
        key = (tuple(norm(m) for m in masses.split(",")), norm(x), norm(y))
        gh.setdefault(key, []).append((fi, t))
    n_gh = sum(len(v) for v in gh.values())
    rows = list(csv.DictReader(open(paper_csv, encoding="utf-8")))
    present = absent = 0
    worst = Decimal(0)
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["table", "Fi", "m1", "m2", "m3", "status", "ll5_Fi", "T_ll5_minus_T_paper"])
        for r in rows:
            key = (tuple(norm(r[k]) for k in ("m1", "m2", "m3")), norm(r["x"]), norm(r["y"]))
            hit = gh.get(key)
            if hit:
                present += 1
                d = Decimal(hit[0][1]) - Decimal(r["T"])
                worst = max(worst, abs(d))
                w.writerow([r["table"], r["Fi"], r["m1"], r["m2"], r["m3"], "present", hit[0][0], d])
            else:
                absent += 1
                w.writerow([r["table"], r["Fi"], r["m1"], r["m2"], r["m3"], "absent", "", ""])
    print(f"authors' table: {n_gh} rows; paper rows: {len(rows)}; present {present}; absent {absent}; "
          f"largest |T difference| among matches {worst}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
