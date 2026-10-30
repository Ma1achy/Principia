"""mh1991_checks.py <tableI_reconciled.csv> <henon_table4_reconciled.csv>: paper-only checks of Mikkola &
Hietarinta (1991) Table I, and its decoding against Henon (1977) Table IV and Schubart (1956).

1. masses: m1 = 1 - a - b, m0 = 1 + 2a, m2 = 1 - a + b with a = 0.01 Ia, b = 0.01 Ib (eq. 2), sum 3;
2. lambda_l = exp(i phi) with k_l = cos(phi) (eq. 3: lambda^2 - 2 k_l lambda + 1 = 0), phi printed to 0.1 deg,
   angles > 180 deg being 360 - arccos (the text's 'clockwise rotation' note);
3. equal-outer-mass rows (Ib = 0) against Henon Table IV (G = 1, masses summing to 1, E = -(1/2) sum m_i m_j,
   period T, stability k1, k2), rescaled to MH's units (masses summing to 3, E = -1): R = Henon x3 * L,
   T_MH = T * sqrt(L^3 / 3) with L = 9 E_H / (-1); Henon's k1, k2 compared with 2 k_l, 2 k_t (Henon uses
   lambda^2 - k lambda + 1). Rows with masses not tabulated by Henon are interpolated (quadratic, 3 nearest
   rows) and marked so. The printed P is compared with T_MH and with T_MH / R^(3/2).
"""
import csv, math, sys
T = list(csv.DictReader(open(sys.argv[1])))
H = [r for r in csv.DictReader(open(sys.argv[2])) if 0 < float(r["m2"]) < 1]
print("1./2. per orbit:")
for r in T:
    a, b = 0.01 * int(r["Ia"]), 0.01 * int(r["Ib"])
    m = (1 - a - b, 1 + 2 * a, 1 - a + b)
    pm = (float(r["m1"]), float(r["m0"]), float(r["m2"]))
    dm = max(abs(x - y) for x, y in zip(m, pm))
    kl = float(r["k_l"])
    ph = math.degrees(math.acos(kl)) if abs(kl) <= 1 else float("nan")
    lam = r["lambda_l_deg"]
    okl = "" if not lam else "%.1f / %.1f" % (ph, 360 - ph)
    print("   (%4s,%4s) masses from (a,b) match printed to %.3f (sum %.2f); arccos(k_l) = %s printed %s" % (
        r["Ia"], r["Ib"], dm, sum(pm), okl or "|k_l|>1 (unstable, no lambda printed)", lam or "-"))
def henon(m13):
    xs = sorted(H, key=lambda h: abs(float(h["m1_eq_m3"]) - m13))[:3]
    exact = abs(float(xs[0]["m1_eq_m3"]) - m13) < 5e-7
    def lag(k):
        if exact: return float(xs[0][k])
        s = 0
        for j, hj in enumerate(xs):
            p = float(hj[k])
            for l, hl in enumerate(xs):
                if l != j: p *= (m13 - float(hl["m1_eq_m3"])) / (float(hj["m1_eq_m3"]) - float(hl["m1_eq_m3"]))
            s += p
        return s
    return exact, lag("minus_x1_eq_x3"), lag("T"), lag("k1"), lag("k2")
print("3. Ib = 0 rows against Henon Table IV:")
for r in T:
    if r["Ib"] != "0": continue
    m1, m0 = float(r["m1"]), float(r["m0"])
    m13 = m1 / 3; m2 = m0 / 3
    exact, x3, Th, k1, k2 = henon(m13)
    EH = -0.5 * (2 * m13 * m2 + m13 * m13)
    L = 9 * EH / -1.0
    R = x3 * L; Tm = Th * math.sqrt(L ** 3 / 3)
    print("   m=(%s,%s,%s) Henon %s: R %.5f (MH %s); T_MH %.4f; T_MH/R^1.5 %.4f (MH P %s); Henon k1/2 %.6f (MH k_l %s); k2/2 %.6f (MH k_t %s)" % (
        r["m1"], r["m0"], r["m2"], "row" if exact else "interpolated", R, r["R"], Tm, Tm / R ** 1.5, r["P"], k1 / 2, r["k_l"], k2 / 2, r["k_t"]))
