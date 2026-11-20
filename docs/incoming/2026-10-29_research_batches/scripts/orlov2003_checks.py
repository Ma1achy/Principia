"""orlov2003_checks.py <table_reconciled.csv>: internal consistency of Orlov, Petrova & Martynova (2003)'s table.

(i) the '1-5' row equals the sum of the n = 1..5 rows (3-decimal rounding: |diff| <= 0.003 expected);
(ii) the paper states the table is symmetric about eps1 = eps2: entry (eps1, eps2) = entry (eps2, eps1); every
asymmetric pair is listed; (iii) fraction escaping after 1-5 passages + stable <= 1 (the remainder disrupts
after n > 5 within t = 1000).
"""
import csv, sys
R = list(csv.DictReader(open(sys.argv[1])))
E = ["0.2", "0.5", "1", "2", "5"]
v = {(r["eps1"], r["n"], e): float(r["e2_" + e]) for r in R for e in E}
w = 0.0
for a in E:
    for b in E:
        s = sum(v[(a, n, b)] for n in "12345"); d = s - v[(a, "1-5", b)]; w = max(w, abs(d))
        if abs(d) > 0.0035: print("(i) eps1=%s eps2=%s: sum n=1..5 = %.3f, '1-5' = %.3f" % (a, b, s, v[(a, "1-5", b)]))
        if v[(a, "1-5", b)] + v[(a, "stable", b)] > 1: print("(iii) eps1=%s eps2=%s exceeds 1" % (a, b))
print("(i) max |sum(n=1..5) - '1-5'| = %.3f over 25 mass pairs" % w)
asym = [(a, b, n, v[(a, n, b)], v[(b, n, a)]) for i, a in enumerate(E) for b in E[i + 1:]
        for n in ["1", "2", "3", "4", "5", "1-5", "stable"] if v[(a, n, b)] != v[(b, n, a)]]
print("(ii) asymmetric entries (eps1, eps2, n, value, mirrored value): %d of %d" % (len(asym), 10 * 7))
for x in asym: print("     %s" % (x,))
