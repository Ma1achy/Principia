"""li2021_table1_precision.py <li2021/table1_reconciled.csv>: the precision each Li et al. 2021 Table 1 row carries.

Table 1 (p. 6) prints each isosceles row's y to 17 significant digits. Some are shorter values printed as the binary
double nearest them: a row is reported as such when the 17-digit print equals the double of a value with fewer
digits (repr of the double, the shortest decimal that round-trips). IT-0 (Lagrange) is compared with the exact
y = sqrt(3)/2, which p. 5 states, with the precision p. 5 states, "about 10^-11".

Exact decimal arithmetic on the transcribed digits; nothing is integrated.
Run: python3 -I scripts/li2021_table1_precision.py li2021/table1_reconciled.csv > li2021/table1_precision.txt
"""
import csv, sys
from decimal import Decimal as D, getcontext
getcontext().prec = 40
for r in csv.DictReader(open(sys.argv[1], encoding="utf-8")):
    printed = r["y"]
    short = repr(float(printed))
    digits = len(D(short).normalize().as_tuple().digits)
    note = ""
    if r["label"] == "IT-0":
        exact = D(3).sqrt() / 2
        note = "; sqrt(3)/2 = %.17f, printed - exact = %.2e" % (exact, D(printed) - exact)
    print("%s: printed y = %s; the double's shortest decimal %s, %d significant figures%s"
          % (r["label"], printed, short, digits, note))
