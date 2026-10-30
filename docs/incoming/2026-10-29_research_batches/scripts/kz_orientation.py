"""kz_orientation.py <table1_reconciled.csv>: which image of REQ-VAL-041's figure-eight start Kapela & Zgliczynski 2003's
box certifies.

The start (Chenciner-Montgomery, unit masses, G = 1; the corpus's digits, principia_listen.md section 12, there at
total mass 1, here at unit masses): x1 = (-0.97000436, 0.24308753), x2 = -x1, x3 = 0; v3 = (-0.93240737, -0.86473146),
v1 = v2 = -v3 / 2. Its energy is the E = -1.2871419918 that principia_dd_validation_orbits.md section 0 quotes.

Kapela & Zgliczynski's frame (eq. 1, p. 1906): E(v, u) = (1, 0, -1, 0, 0, 0, v, u, v, u, -2v, -2u), an outer body at
(+1, 0) moving at (v, u), the other at (-1, 0) at the same (v, u), the middle body at 0. Putting the start in that
frame takes a rotation (an outer body onto the +x axis), a rescale r -> r / |x1| (so velocities -> v * |x1|^(1/2),
the similarity r -> lam r, t -> lam^(3/2) t), and possibly a reflection y -> -y. Four images: x1 or x2 on +x, each
with or without the reflection. Each is checked against the box N(x0, [X]) of Table 1.

Exact decimal arithmetic; nothing is integrated.
Run: python3 -I scripts/kz_orientation.py kapela_zgliczynski_2003/table1_reconciled.csv > kapela_zgliczynski_2003/orientation_check.txt
"""
import csv, sys
from decimal import Decimal as D, getcontext
getcontext().prec = 40
rows = {(r["quantity"], r["component"]): r for r in csv.DictReader(open(sys.argv[1]))}
box = {c: (D(rows[("N(x0;[X])", c)]["lo"]), D(rows[("N(x0;[X])", c)]["hi"])) for c in ("v", "u")}

x1 = (D("-0.97000436"), D("0.24308753"))
v3 = (D("-0.93240737"), D("-0.86473146"))
v1 = (-v3[0] / 2, -v3[1] / 2)
x2, v2 = (-x1[0], -x1[1]), v1
r12 = (4 * (x1[0] ** 2 + x1[1] ** 2)).sqrt()
r13 = (x1[0] ** 2 + x1[1] ** 2).sqrt()
ke = (v1[0] ** 2 + v1[1] ** 2 + v2[0] ** 2 + v2[1] ** 2 + v3[0] ** 2 + v3[1] ** 2) / 2
print("start: E = %.10f (principia_dd_validation_orbits.md section 0 quotes -1.2871419918)" % (ke - 1 / r12 - 2 / r13))
lam = r13
print("|x1| = %.10f; rescale: positions / |x1|, velocities * |x1|^(1/2) = %.10f" % (lam, lam.sqrt()))

for name, xo, vo in (("x1 on +x", x1, v1), ("x2 on +x", x2, v2)):
    c, s = xo[0] / lam, xo[1] / lam
    rot = lambda w: (c * w[0] + s * w[1], -s * w[0] + c * w[1])
    xr = rot(xo)
    vr = rot(vo)
    for refl in (False, True):
        v, u = vr[0] * lam.sqrt(), (-vr[1] if refl else vr[1]) * lam.sqrt()
        out = []
        for comp, val in (("v", v), ("u", u)):
            lo, hi = box[comp]
            mid, half = (lo + hi) / 2, (hi - lo) / 2
            out.append("%s %+.3e from the centre, half-width %.3e, inside %s" % (comp, val - mid, half, lo <= val <= hi))
        print("%s%s: outer body at (%.10f, %.1e) -> (+1, 0); (v, u) = (%.10f, %.10f): %s"
              % (name, ", reflected" if refl else "", xr[0] / lam, xr[1] / lam, v, u, "; ".join(out)))
