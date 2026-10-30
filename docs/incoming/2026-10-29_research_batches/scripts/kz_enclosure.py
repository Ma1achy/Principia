"""kz_enclosure.py <table1_reconciled.csv> [threebodyorbits_ics.csv]: arithmetic checks on Kapela & Zgliczynski 2003's certified enclosure.

Exact decimal arithmetic on the transcribed numbers; nothing is integrated.
  1. the interval-Newton image N(x0, [X]) lies inside [X] = [x0 - delta, x0 + delta] (x0, delta from
     the text, p. 1906), which is the assertion theorem 2.1 needs;
  2. the printed diam N(x0, [X]) agrees with hi - lo of the printed intervals;
  3. the period implied by the convexity proof's step count (§3.2: h = 0.01, 53 steps to cross the
     section; T = 12 T~, lemma 3.3): 52 h < T~ <= 53 h if the steps are all h, so 6.24 < T <= 6.36.
     The paper prints no period; this bound is derived here and is coarse.
  4. the full initial condition E(v, u) = (1, 0, -1, 0, 0, 0, v, u, v, u, -2v, -2u) at the box centre.
  5. the energy range over the box against the figure-eight energy the validation-orbits doc quotes;
  6. (if the catalogue CSV is given) the threebodyorbits V.1.A row, published (pub_*) and refined (ref_*),
     against the box: same frame (bodies at (-1, 0), (1, 0), (0, 0), unit masses, G = 1), compared only.
"""
import csv, sys
from decimal import Decimal as D, getcontext
getcontext().prec = 40
rows = {(r["quantity"], r["component"]): r for r in csv.DictReader(open(sys.argv[1]))}
x0 = {"v": D("0.347116768716"), "u": D("0.532724944657")}
delta = D("1e-6")
for c in ("v", "u"):
    lo, hi = D(rows[("N(x0;[X])", c)]["lo"]), D(rows[("N(x0;[X])", c)]["hi"])
    inside = x0[c] - delta <= lo and hi <= x0[c] + delta
    w = hi - lo
    printed = D(rows[("diam N(x0;[X])", c)]["lo"])
    print("%s: N = [%s, %s]  inside [x0 +- 1e-6] = [%s, %s]: %s; width %s, printed diam %s, |diff| %.1e;"
          " midpoint %s; x0 - midpoint %.3e" % (c, lo, hi, x0[c] - delta, x0[c] + delta, inside, w, printed,
                                                abs(w - printed), (lo + hi) / 2, x0[c] - (lo + hi) / 2))
print("period: not printed; from 53 steps of h = 0.01 to the section, T = 12 T~ in (%s, %s] (derived, coarse)" % (
    12 * 52 * D("0.01"), 12 * 53 * D("0.01")))
v = (D(rows[("N(x0;[X])", "v")]["lo"]) + D(rows[("N(x0;[X])", "v")]["hi"])) / 2
u = (D(rows[("N(x0;[X])", "u")]["lo"]) + D(rows[("N(x0;[X])", "u")]["hi"])) / 2
print("E(v, u) at the centre of N: x1, y1, x2, y2, x3, y3, vx1, vy1, vx2, vy2, vx3, vy3 =")
print("  1, 0, -1, 0, 0, 0, %s, %s, %s, %s, %s, %s" % (v, u, v, u, -2 * v, -2 * u))
# 5. Energy over the enclosure, G = 1, unit masses (the paper's eq. 1 normalisation): bodies at (+-1, 0) and the
#    origin, velocities (v, u), (v, u), (-2v, -2u); E = 3 (v^2 + u^2) - (1 + 1 + 1/2). E is monotone in v and u
#    over the box (both positive), so its range is attained at the corners. Compared with the figure-eight
#    energy the validation-orbits design doc quotes (docs/design/principia_dd_validation_orbits.md §0):
#    E = -1.2871419918 (10 decimals).
lo_v, hi_v = D(rows[("N(x0;[X])", "v")]["lo"]), D(rows[("N(x0;[X])", "v")]["hi"])
lo_u, hi_u = D(rows[("N(x0;[X])", "u")]["lo"]), D(rows[("N(x0;[X])", "u")]["hi"])
E = lambda a, b: 3 * (a * a + b * b) - D("2.5")
e_lo, e_hi, e_vo = E(lo_v, lo_u), E(hi_v, hi_u), D("-1.2871419918")
print("E over the enclosure: [%.12f, %.12f]; VO section 0 quotes %s: inside %s (to its 10 decimals: %s)"
      % (e_lo, e_hi, e_vo, e_lo <= e_vo <= e_hi,
         e_lo.quantize(D("1e-10")) <= e_vo <= e_hi.quantize(D("1e-10"))))
print("VO section 0 period T = 6.32591398 lies in the derived (6.24, 6.36]: %s" % (D("6.24") < D("6.32591398") <= D("6.36")))

if len(sys.argv) > 2:
    cat = {r["code"]: r for r in csv.DictReader(open(sys.argv[2]))}["V.1.A"]
    for tag in ("pub", "ref"):
        cv, cu = D(cat[tag + "_vx1"]), D(cat[tag + "_vy1"])
        print("threebodyorbits V.1.A %s_* (vx1, vy1) = (%s, %s): v inside %s (%s), u inside %s (%s)" % (
            tag, cv, cu, lo_v <= cv <= hi_v, "%+.3e beyond" % (lo_v - cv if cv < lo_v else cv - hi_v) if not lo_v <= cv <= hi_v else "",
            lo_u <= cu <= hi_u, "%+.3e beyond" % (lo_u - cu if cu < lo_u else cu - hi_u) if not lo_u <= cu <= hi_u else ""))
