"""henon_checks.py <henon_dir> <schubart_table3_reconciled.csv>: paper-only checks of Henon (1977) Tables I-IV.

Per positive-mass row (any row with a zero mass is a limit row and is skipped): (3) m1 + m2 + m3 = 1;
x2, u2 from (4); order (2) x1 <= x2 <= x3; energy residual against (5) E = -(m2 m3 + m3 m1 + m1 m2) / 2.
Then the equal-mass row of Table IV against Schubart (1956) after rescaling (G = 1 in both; Henon's masses
1/3, energy -1/6, origin of time u1 = u3; Schubart's masses 1, length unit r13 at the 1-2 collision).
No integration; arithmetic on printed numbers only.
"""
import csv, math, os, sys

def rows(p):
    return list(csv.DictReader(open(p)))

def check(name, R, sym):
    worst_m = worst_e = 0.0
    n = 0
    for r in R:
        if sym:
            m1 = m3 = float(r["m1_eq_m3"]); m2 = float(r["m2"]); x3 = float(r["minus_x1_eq_x3"]); x1 = -x3
        else:
            m1, m2, m3 = float(r["m1"]), float(r["m2"]), float(r["m3"]); x1, x3 = float(r["x1"]), float(r["x3"])
        if min(m1, m2, m3) == 0.0:
            continue
        u = float(r["u1_eq_u3"])
        x2 = -(m1 * x1 + m3 * x3) / m2
        u2 = -(m1 + m3) * u / m2
        assert x1 <= x2 <= x3, (name, r)
        T = 0.5 * (m1 * u * u + m2 * u2 * u2 + m3 * u * u)
        V = m2 * m3 / (x3 - x2) + m3 * m1 / (x3 - x1) + m1 * m2 / (x2 - x1)
        E0 = -0.5 * (m2 * m3 + m3 * m1 + m1 * m2)
        dm, de = abs(m1 + m2 + m3 - 1), abs((T - V) - E0)
        worst_m, worst_e = max(worst_m, dm), max(worst_e, de)
        n += 1
        if de > 2e-5:
            print("   %s row m=(%g,%g,%g): energy residual %.2e" % (name, m1, m2, m3, de))
    print("%s: %d positive-mass rows; max |sum m - 1| = %.1e; max |E - E(5)| = %.1e; order (2) holds" % (name, n, worst_m, worst_e))

d = sys.argv[1]
check("Table I", rows(os.path.join(d, "table1_eye.csv")), False)
check("Table II", rows(os.path.join(d, "table2_eye.csv")), False)
check("Table III", rows(os.path.join(d, "table3_eye.csv")), False)
check("Table IV", rows(os.path.join(d, "table4_eye.csv")), True)

print("Shared rows (the same orbit printed in two tables):")
T1 = {r["m1"]: r for r in rows(os.path.join(d, "table1_eye.csv"))}
T2 = {r["m1"]: r for r in rows(os.path.join(d, "table2_eye.csv"))}
for a, b in (("0.650504", "0.162706"),):
    r, s = T1[a], T2[b]
    same = (r["x1"].lstrip("-") == s["x3"]) and (r["x3"] == s["x1"].lstrip("-")) and r["u1_eq_u3"] == s["u1_eq_u3"] and r["T"] == s["T"]
    print("   doubly critical orbit (13): Table I m1=%s and Table II m1=%s mirror each other (x1<->x3): %s" % (a, b, same))

print("Schubart (1956) vs Table IV equal-mass row (m1 = m3 = 0.333333, m2 = 0.333334):")
h = [r for r in rows(os.path.join(d, "table4_eye.csv")) if r["m1_eq_m3"] == "0.333333"][0]
ES, TS, R13 = -(4.91610 / 4 + 2), 0.21470, 1.22949   # Schubart p. 22, decoded in schubart_1956/decode_check.txt
mu, EH = 1 / 3, -1 / 6
lam = mu * mu * ES / EH            # length scale: E ~ mu^2 / lam
tau = math.sqrt(lam ** 3 / mu)     # time scale: G mu tau^2 / lam^3 = 1
vs = math.sqrt((2 / (R13 / 2) + 1 / R13 + ES) / 3)
print("   length scale %.6f, time scale %.6f" % (lam, tau))
print("   x3:  Schubart r13max/2 * scale = %.6f ; Henon %s" % (R13 / 2 * lam, h["minus_x1_eq_x3"]))
print("   u1:  Schubart -v1 * lam/tau   = %.6f ; Henon %s" % (-vs * lam / tau, h["u1_eq_u3"]))
print("   T:   Schubart 4 t*   * tau     = %.6f ; Henon %s" % (4 * TS * tau, h["T"]))
print("   Schubart's stated values carry 5 decimals: t* +-5e-6 -> T +-%.1e; r13 +-5e-6 -> x3 +-%.1e" % (4 * 5e-6 * tau, 5e-6 / 2 * lam))
