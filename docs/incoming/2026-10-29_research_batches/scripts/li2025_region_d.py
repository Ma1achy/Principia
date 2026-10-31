"""li2025_region_d.py <li2025/table1_reconciled.csv>: which of Li et al. 2025's Table 1 starts lie in region D.

Region D (Li et al. 2021 section 2.1; Tanikawa, Saito & Mikkola 2019 p. 9; Tanikawa & Mikkola 2021 eq. 2):
D = {(x, y) : x >= 0, y >= 0, (x + 0.5)^2 + y^2 <= 1}, body 2 at (-0.5, 0), body 3 at (0.5, 0), body 1 at (x, y).
REQ-VAL-019's Anosova entry refuses a point outside D, so a start outside D cannot enter that way.

Exact decimal arithmetic on the transcribed digits; nothing is integrated.
Run: python3 -I scripts/li2025_region_d.py li2025/table1_reconciled.csv > li2025/region_d_check.txt
"""
import csv, sys
from decimal import Decimal as D
inside = 0
rows = list(csv.DictReader(open(sys.argv[1], encoding="utf-8")))
for r in rows:
    x, y = D(r["x"]), D(r["y"])
    q = (x + D("0.5")) ** 2 + y ** 2
    ok = x >= 0 and y >= 0 and q <= 1
    inside += ok
    print("(m1, m2) = (%s, %s): x = %.8f, y = %.8f; (x + 0.5)^2 + y^2 = %.4f; in D: %s" % (r["m1"], r["m2"], x, y, q, ok))
print("in D: %d of %d; outside D: %d" % (inside, len(rows), len(rows) - inside))
