"""secondary_exclusion.py <yasko_table_reconciled.csv> <iasko_table_reconciled.csv> <threebodyorbits_ics.csv>

Overlap of Yasko & Orlov 2015 and Iasko & Orlov 2014 with the threebodyorbits snapshot (no integration).
Both papers use G = 1 and unit masses and quote periods in units of the mean crossing time
tau = G M^(5/2) / (2|E|)^(3/2), M = 3 (Yasko 2015 Sec. 2; Iasko 2014 uses the units of its refs [8, 12]).

Yasko (free fall, region D): bodies A, B at (-0.5, 0), (0.5, 0), C at (xi, eta), at rest. Catalogue candidates:
equal unit masses, all velocities zero, two bodies at (-0.5, 0), (0.5, 0); the free body compared with (xi, eta)
and its mirror (-xi, eta). Yasko's points are region centres on a 1e-4 grid accepted at Phi < 0.03, not exact
initial conditions, so the matching rule is distance < 1e-3 together with |T/tau difference| < 0.05; every
nearest neighbour is listed.

Iasko (syzygy, (k, phi)): catalogue candidates are equal unit masses with the bodies at (-1, 0), (1, 0), (0, 0)
and velocities (p1, p2), (p1, p2), (-2 p1, -2 p2); k' = 1.2 (p1^2 + p2^2), phi' = arctan(p2 / p1) (Iasko's own
conversion, p. 870), folded to [0, pi/2] by the reflection symmetries. Matching rule: |dk| < 0.005 and
|dphi| < 0.01 (the table's 3 decimals plus the region size) together with |T/tau difference| < 0.05.
"""
import csv, math, sys

Y = list(csv.DictReader(open(sys.argv[1])))
I = [r for r in csv.DictReader(open(sys.argv[2])) if r["k"] != "-"]
C = [r for r in csv.DictReader(open(sys.argv[3])) if r["m1"] == r["m2"] == r["m3"] == "1.0"]
f = lambda r, k: float(r[k]) if r[k] not in ("", None) else float("nan")
tau = lambda E: 3 ** 2.5 / (2 * abs(E)) ** 1.5

ff = []
for r in C:
    if any(f(r, "pub_v%s%d" % (c, k)) != 0 for c in "xy" for k in (1, 2, 3)):
        continue
    pts = [(f(r, "pub_x%d" % k), f(r, "pub_y%d" % k)) for k in (1, 2, 3)]
    free = [p for p in pts if not (abs(abs(p[0]) - 0.5) < 1e-12 and p[1] == 0)]
    if len(free) == 1:
        x, y = free[0]
        U = 1 / 1.0 + 1 / math.hypot(x + 0.5, y) + 1 / math.hypot(x - 0.5, y)
        ff.append((r["key"], x, y, f(r, "T") / tau(-U)))
print("Yasko 2015: %d catalogue free-fall rows with equal unit masses" % len(ff))
nm = 0
for r in Y:
    xi, eta = float(r["xi"]), float(r["eta"])
    U = 1 + 1 / math.hypot(xi + 0.5, eta) + 1 / math.hypot(xi - 0.5, eta)
    d, key, Tc = min((math.hypot(s * x - xi, y - eta), k, T) for k, x, y, T in ff for s in (1, -1))
    hit = d < 1e-3 and abs(Tc - float(r["T_tau"])) < 0.05
    nm += hit
    print("   Y%-2s (%.4f, %.4f) T=%s tau | nearest %-26s dist %.2e  T=%.3f tau %s" % (r["N"], xi, eta, r["T_tau"], key, d, Tc, "MATCH" if hit else ""))
print("   matches: %d of %d" % (nm, len(Y)))

sv = []
for r in C:
    x = [f(r, "pub_x%d" % k) for k in (1, 2, 3)]; y = [f(r, "pub_y%d" % k) for k in (1, 2, 3)]
    vx = [f(r, "pub_vx%d" % k) for k in (1, 2, 3)]; vy = [f(r, "pub_vy%d" % k) for k in (1, 2, 3)]
    if sorted(x) != [-1.0, 0.0, 1.0] or any(v != 0 for v in y):
        continue
    c = x.index(0.0); o = [i for i in range(3) if i != c]
    if not (vx[o[0]] == vx[o[1]] and vy[o[0]] == vy[o[1]] and abs(vx[c] + 2 * vx[o[0]]) < 1e-9 and abs(vy[c] + 2 * vy[o[0]]) < 1e-9):
        continue
    p1, p2 = vx[o[0]], vy[o[0]]
    E = 3 * (p1 * p1 + p2 * p2) - 2.5
    ph = math.atan(abs(p2 / p1)) if p1 != 0 else math.pi / 2
    sv.append((r["key"], 1.2 * (p1 * p1 + p2 * p2), ph, f(r, "T") / tau(E)))
print("Iasko 2014: %d catalogue rows in the syzygy form (k', phi' computable)" % len(sv))
nm = 0
for r in I:
    k, ph, T = float(r["k"]), float(r["phi"]), float(r["T_tau"])
    best = min(sv, key=lambda s: ((s[1] - k) / 0.005) ** 2 + ((s[2] - ph) / 0.01) ** 2 + ((s[3] - T) / 0.05) ** 2)
    hit = abs(best[1] - k) < 0.005 and abs(best[2] - ph) < 0.01 and abs(best[3] - T) < 0.05
    nm += hit
    print("   I%-2s k=%.3f phi=%.3f T=%.2f tau | nearest %-28s k'=%.4f phi'=%.4f T=%.3f tau %s %s" % (
        r["orbit"], k, ph, T, best[0], best[1], best[2], best[3], "MATCH" if hit else "", r["notes"] if r["notes"] != "-" else ""))
print("   matches: %d of %d" % (nm, len(I)))
