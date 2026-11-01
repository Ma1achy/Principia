"""moore2006_checks.py <table3_reconciled.csv> <ics_reconciled.csv> <threebodyorbits_ics.csv>

Paper arithmetic on Moore & Nauenberg's criss-cross orbit (p. 10-11), no integration:
1. the printed initial conditions against the Fourier series of Table 3 at t = 0 (eq. 11: x_i = sum a_ik cos kt,
   y_i = sum b_ik sin kt; body 2 from the symmetry a2k = +-b1k, b2k = +-a1k, + for k = 3, 7, 11, 15, - for
   k = 1, 5, 9, 13; body 3: b3k = +-a3k likewise);
2. centre of mass, momentum, energy E and angular momentum L with masses 1, G = 1 (the action of p. 3 uses
   unit masses and G = 1) and period T = 2 pi (p. 4: 'we choose T = 2 pi');
3. overlap with data we already have, the threebodyorbits snapshot: equal unit-mass rows, compared by the scale invariants
   T |E|^(3/2) and L^2 |E| (mirror images have the same values). Matching rule: both within 1e-3 relative
   (the printed coefficients converge to 5 decimals).
"""
import csv, math, sys
A = list(csv.DictReader(open(sys.argv[1]))); I = list(csv.DictReader(open(sys.argv[2])))
sgn = lambda k: 1 if k % 4 == 3 else -1
k = [int(r["k"]) for r in A]
a1 = [float(r["a1k"]) for r in A]; b1 = [float(r["b1k"]) for r in A]; a3 = [float(r["a3k"]) for r in A]
x1, vy1 = sum(a1), sum(kk * b for kk, b in zip(k, b1))
x2 = sum(sgn(kk) * b for kk, b in zip(k, b1)); vy2 = sum(kk * sgn(kk) * a for kk, a in zip(k, a1))
x3, vy3 = sum(a3), sum(kk * sgn(kk) * a for kk, a in zip(k, a3))
P = {int(r["body"]): r for r in I}
print("1. Fourier at t = 0 vs printed ICs:")
for b, (x, vy) in {1: (x1, vy1), 2: (x2, vy2), 3: (x3, vy3)}.items():
    print("   body %d: x %.5f (printed %s), vy %.5f (printed %s)" % (b, x, P[b]["x"], vy, P[b]["vy"]))
q = [(float(P[b]["x"]), 0.0) for b in (1, 2, 3)]; v = [(0.0, float(P[b]["vy"])) for b in (1, 2, 3)]
K = 0.5 * sum(a * a + c * c for a, c in v)
U = -sum(1 / math.dist(q[i], q[j]) for i in range(3) for j in range(i + 1, 3))
E = K + U; L = sum(p[0] * w[1] - p[1] * w[0] for p, w in zip(q, v)); T = 2 * math.pi
print("2. sum x = %.5f, sum vy = %.5f; E = %.6f, L = %.6f, T = 2 pi; invariants T|E|^1.5 = %.5f, L^2|E| = %.5f" % (
    sum(p[0] for p in q), sum(w[1] for w in v), E, L, T * abs(E) ** 1.5, L * L * abs(E)))
best = []
for r in csv.DictReader(open(sys.argv[3])):
    if not (r["m1"] == r["m2"] == r["m3"] == "1.0"): continue
    try:
        Ec, Lc, Tc = float(r["E_recomp"] or r["E_cat"]), float(r["L_recomp"] or r["L_cat"]), float(r["T"])
    except ValueError:
        continue
    if Ec >= 0: continue
    i1, i2 = Tc * abs(Ec) ** 1.5, Lc * Lc * abs(Ec)
    d = max(abs(i1 / (T * abs(E) ** 1.5) - 1), abs(i2 / (L * L * abs(E)) - 1))
    best.append((d, r["key"], i1, i2))
best.sort()
print("3. closest catalogue rows by the invariants (relative difference, key, T|E|^1.5, L^2|E|):")
for b in best[:5]: print("   %.2e %-40s %.5f %.5f" % b)
print("   matches (both within 1e-3): %d" % sum(1 for b in best if b[0] < 1e-3))
