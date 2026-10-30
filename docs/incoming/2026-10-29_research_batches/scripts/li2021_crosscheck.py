"""li2021_crosscheck.py <table2_reconciled.csv> <li2021_xlsx_ics.csv>: the paper's Table 2 against the workbook.

For each Table 2 row, finds the workbook row with the same label and compares x, y, T as decimals:
the printed value (16 significant digits) is rounded to the number of significant digits the workbook
stores for that cell (LibreOffice wrote at most 15), and the two strings must then be equal. Also
prints the absolute difference of the unrounded values.
"""
import csv, sys
from decimal import Decimal, ROUND_HALF_EVEN

def sig(s):
    m = s.lower().split("e")[0].lstrip("-").replace(".", "").lstrip("0")
    return len(m)

def round_sig(d, n):
    if d == 0: return d
    e = d.adjusted() - n + 1
    return d.quantize(Decimal(1).scaleb(e), rounding=ROUND_HALF_EVEN)

t2 = list(csv.DictReader(open(sys.argv[1])))
wb = {r["label"]: r for r in csv.DictReader(open(sys.argv[2]))}
bad = 0
for r in t2:
    w = wb.get(r["label"])
    if w is None:
        print(r["label"], "not in workbook"); bad += 1; continue
    for k in ("x", "y", "T"):
        p, q = Decimal(r[k]), Decimal(w[k])
        n = sig(w[k])
        ok = round_sig(p, n) == round_sig(q, n)
        bad += not ok
        print("%-8s %s printed %s workbook %s (%d sig) rounded-equal %s |diff| %.1e" % (
            r["label"], k, r[k], w[k], n, ok, abs(p - q)))
print("rows %d, cells %d, mismatches %d" % (len(t2), 3 * len(t2), bad))
