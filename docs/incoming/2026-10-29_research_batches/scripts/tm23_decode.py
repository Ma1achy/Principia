"""tm23_decode.py <supp_reconciled.csv>: Tanikawa & Mikkola 2023's (alpha2, beta1) parametrisation, decoded.

From the paper (CeMDA 135, 54), §2.1-2.2 and §5.1, §6.1 (quoted in the README):
  bodies m1 = m2 = m3 = 1, G = 1 (unit coefficient in eq. 2); body 1 at (x, y), body 2 at (-0.5, 0),
  body 3 at (0.5, 0); R_k = X_k - G, G = (X1 + X2 + X3)/3 (eq. 22), so
  R1 = (2/3)(x, y), R2 = -(1/3)(x, y) - (0.5, 0), R3 = -(1/3)(x, y) + (0.5, 0)  (eq. 23);
  V1 = -2 alpha2 R1 + beta1 R2,  V2 = alpha2 R1 - 2 beta1 R2,  V3 = -V1 - V2  (eqs. 20-22).
Velocities are barycentric (the paper integrates in "an inertial barycentric reference system").
Table 8 adds t and s. §6.1 (main text) writes V1 = t(-2 alpha2 R1) + s beta1 R2,
V2 = t(alpha2 R1) + s(-2 beta1 R2), i.e. alpha2 -> t alpha2, beta1 -> s beta1; the Table 8 caption
writes V1 = (1 - t)(-2 alpha2) R1 + t beta1 R2, V2 = (1 - t) alpha2 R1 + t(-2 beta1) R2. The two
disagree; this script tests both against rows of Tables 2 and 6 that must be the same TCP.

Checks (exact rational arithmetic on the printed decimals): for every row, sum of m V = 0 and total
angular momentum sum R x V = 0; prints one worked conversion; then the Table 8 cross-checks.
No orbit is integrated.
"""
import csv, sys
from fractions import Fraction as F

def state(x, y, a2, b1):
    x, y, a2, b1 = map(F, (x, y, a2, b1))
    R1 = (F(2, 3) * x, F(2, 3) * y)
    R2 = (-x / 3 - F(1, 2), -y / 3)
    R3 = (-x / 3 + F(1, 2), -y / 3)
    V1 = (-2 * a2 * R1[0] + b1 * R2[0], -2 * a2 * R1[1] + b1 * R2[1])
    V2 = (a2 * R1[0] - 2 * b1 * R2[0], a2 * R1[1] - 2 * b1 * R2[1])
    V3 = (-V1[0] - V2[0], -V1[1] - V2[1])
    return (R1, R2, R3), (V1, V2, V3)

rows = list(csv.DictReader(open(sys.argv[1])))
bad = 0
for r in rows:
    a2, b1 = r["alpha2"].replace("−", "-"), r["beta1"].replace("−", "-")
    if r["table"] == "8":   # main-text reading (§6.1)
        a2, b1 = F(r["t"]) * F(a2), F(r["s"]) * F(b1)
    R, V = state(r["x"], r["y"], a2, b1)
    P = (sum(v[0] for v in V), sum(v[1] for v in V))
    L = sum(q[0] * v[1] - q[1] * v[0] for q, v in zip(R, V))
    Rs = (sum(q[0] for q in R), sum(q[1] for q in R))
    bad += (P != (0, 0)) or (L != 0) or (Rs != (0, 0))
print("rows %d; rows with nonzero total momentum, angular momentum or barycentre: %d" % (len(rows), bad))

r = next(r for r in rows if r["table"] == "2" and r["alpha2"] == "0.1")
R, V = state(r["x"], r["y"], r["alpha2"], r["beta1"])
print("worked example, Table 2 row x=%s y=%s alpha2=%s beta1=%s:" % (r["x"], r["y"], r["alpha2"], r["beta1"]))
for k in range(3):
    print("  body %d  R = (%.10f, %.10f)  V = (%.10f, %.10f)" % (k + 1, *map(float, R[k]), *map(float, V[k])))
print("  |V1| / |R1| = %.6f (the paper, §5.2: |V1| = |R1|/100 at alpha2 = 0.005; here 2 alpha2 = 0.2)" % (
    float((V[0][0] ** 2 + V[0][1] ** 2) / (R[0][0] ** 2 + R[0][1] ** 2)) ** 0.5))

def find(tab, a2, b1):
    out = []
    for q in rows:
        if q["table"] == tab and F(q["alpha2"].replace("−", "-")) == a2 and F(q["beta1"].replace("−", "-")) == b1:
            out.append((q["block"], q["x"], q["y"]))
    return out

print("Table 8 against Tables 2 and 6 (same effective (alpha2, beta1) must give the same TCP):")
for r in rows:
    if r["table"] != "8":
        continue
    t, s, a2, b1 = map(F, (r["t"], r["s"], r["alpha2"], r["beta1"]))
    for name, (ea, eb) in (("main text  t*a2, s*b1", (t * a2, s * b1)),
                           ("caption (1-t)*a2, t*b1", ((1 - t) * a2, t * b1))):
        hits = find("2", ea, eb) + find("6", ea, eb)
        for blk, x, y in hits:
            d = max(abs(F(x) - F(r["x"])), abs(F(y) - F(r["y"])))
            print("  T8 (%s, %s) t=%s s=%s a2=%s b1=%s | %s -> (a2, b1) = (%s, %s) | %s (%s, %s) max|diff| %.1e" % (
                r["x"], r["y"], r["t"], r["s"], r["alpha2"], r["beta1"], name, float(ea), float(eb), blk, x, y, float(d)))
