"""mh1989_checks.py <dir>: internal consistency of Mikkola & Hietarinta (1989) Tables 1-4 (paper arithmetic only).

Table 1 and 2 rows are percentage distributions over ten z-bins: each should sum to ~100. Table 2's 'all' row
should be the case-weighted mean of the T=0 (5323 cases) and T=random (12347) rows (caption: totals 5323,
12347, 17670). Table 4's 'true' column should equal Table 3's one-directional t1..t7 (Table 4's caption says
'Table 2 data'). Table 4's model columns: T_k = T_e ln(8/(8-k)) and T_k = T_1/2 tan(k pi/16); T_e and T_1/2
are not printed, so each is solved from k = 4 and the other rows are checked against it.
"""
import csv, math, os, sys
d = sys.argv[1]
rd = lambda n: list(csv.DictReader(open(os.path.join(d, n))))
Z = ["z0.%d5" % i for i in range(10)]
for n, key in (("table1_reconciled.csv", "theta_bin"), ("table2_reconciled.csv", "group")):
    for r in rd(n):
        if r[key] != "sigma":
            print("%s %-9s sum = %.1f" % (n.split("_")[0], r[key], sum(float(r[z]) for z in Z)))
t2 = {r["group"]: r for r in rd("table2_reconciled.csv")}
mx = max(abs((5323 * float(t2["T=0"][z]) + 12347 * float(t2["T=random"][z])) / 17670 - float(t2["all"][z])) for z in Z)
print("table2 'all' vs case-weighted mean of T=0 and T=random: max |diff| = %.2f (percentage points)" % mx)
t3 = rd("table3_reconciled.csv")[0]; t4 = rd("table4_reconciled.csv")
print("table4 'true' == table3 one-directional t1..t7: %s" % all(r["true"] == t3["t%d" % int(r["k"])] for r in t4))
Te = float(t4[3]["Tk_poisson"]) / math.log(2); Th = float(t4[3]["Tk_rational"]) / math.tan(4 * math.pi / 16)
print("T_e = %.2f (from k=4), T_1/2 = %.2f (from k=4)" % (Te, Th))
for r in t4:
    k = int(r["k"])
    print("   k=%d  Te*ln(8/(8-k)) = %7.1f (printed %s)   T1/2*tan(k pi/16) = %7.1f (printed %s)" % (
        k, Te * math.log(8 / (8 - k)), r["Tk_poisson"], Th * math.tan(k * math.pi / 16), r["Tk_rational"]))
