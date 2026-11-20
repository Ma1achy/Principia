"""horizons_integrator_units.py: the statistical targets' stops and horizons in Principia's integrator units.

Principia integrates in M = sum(m_i) = 1, G = 1, I = 1 (integrator contract Part 5), with one global horizon T per
map, T in [0.4, 200] (R-610). A start given with G = 1 and total mass Mu is first brought to M = 1 (masses / Mu,
positions kept, so times x sqrt(Mu)), then rescaled to I = 1 by the inverse-encode contract's Part 2:
I_in = sum(m_i |r_i - r_com|^2) with the M = 1 masses, t_canon = t_in / I_in^(3/4), r_canon = r_in / sqrt(I_in).
So a source time t maps to t * sqrt(Mu) / I_in^(3/4), per start.

1. Orlov, Petrova & Martynova 2002 (p. 933): unit masses, G = 1, h = -1; r = rho = 2.5 (1 - k) (eq. 6), the central
   body at the centre of mass; k = 0.01(0.01)0.99 (eq. 10); stop at t_c = 100 or R_c = 30.
2. Orlov, Petrova & Martynova 2003 (pp. 258-260): m0 = 1 (central), m1 = eps1 (left), m2 = eps2 (right), E = -1,
   G = 1; r = rho = (2 (eps1 + eps2) + eps1 eps2) / 2 * (1 - k) (eq. 10); eps in {0.2, 0.5, 1, 2, 5} (eq. 14),
   k = 0.05(0.05)0.95 (eq. 15); stop at t > 1000 or R > 100.
3. Mikkola & Hietarinta 1989 (pp. 2-3): unit masses, G = 1 (eq. 1), E = -1; q1 = q2 = R at the start, so the
   central body is at the centre of mass; R_nu = (nu - 1/2) x 0.025, nu = 1..100 (eq. 7); Table 3's lifetimes.
4. Martynova & Orlov 2014 (p. 759): unit masses, G = 1, at rest; T_cr = sqrt(G) M^(5/2) / (2|E|)^(3/2), a time, so
   T_cr in integrator units is (2|E_canon|)^(-3/2) with E_canon at M = G = I = 1. Over the interior grid of D at
   step 0.001 (boundaries excluded, as freefall_checks.py counts it), bodies at A(-0.5, 0), B(0.5, 0), C(xi, eta).

Pure arithmetic; nothing is integrated.
Run: python3 -I scripts/horizons_integrator_units.py > horizons_integrator_units.txt
"""
import math

CAP = 200.0


def to_canon(t, masses, xs):
    """Source time t (G = 1, these masses, these collinear or planar positions) in integrator units."""
    mu = sum(masses)
    m = [mi / mu for mi in masses]
    cx = sum(mi * p[0] for mi, p in zip(m, xs))
    cy = sum(mi * p[1] for mi, p in zip(m, xs))
    i_in = sum(mi * ((p[0] - cx) ** 2 + (p[1] - cy) ** 2) for mi, p in zip(m, xs))
    return t * math.sqrt(mu) / i_in ** 0.75, i_in


print("1. Orlov et al. 2002: stop t_c = 100 (and R_c = 30), unit masses, h = -1")
over = []
rows = []
for n in range(1, 100):
    k = n / 100
    r = 2.5 * (1 - k)
    tc, i_in = to_canon(100.0, (1, 1, 1), ((-r, 0), (0, 0), (r, 0)))
    rows.append((k, r, i_in, tc, 30 / math.sqrt(i_in)))
    if tc > CAP:
        over.append(k)
for k, r, i_in, tc, rc in rows:
    if round(k * 100) in (1, 10, 21, 50, 55, 56, 90, 99):
        print("   k = %.2f: r = rho = %.4f, I (M = 1) = %.4e; t_c = 100 -> %.4g; R_c = 30 -> %.4g" % (k, r, i_in, tc, rc))
print("   t_c past the cap of 200 for %d of the 99 k values: k = %.2f to %.2f" % (len(over), min(over), max(over)))
print("   range of t_c in integrator units over the grid: %.4g to %.4g" % (min(x[3] for x in rows), max(x[3] for x in rows)))

print("2. Orlov et al. 2003: stop t > 1000 (and R > 100), m0 = 1, E = -1")
eps = (0.2, 0.5, 1, 2, 5)
cells = []
for e1 in eps:
    for e2 in eps:
        for n in range(1, 20):
            k = n * 0.05
            r = (2 * (e1 + e2) + e1 * e2) / 2 * (1 - k)
            tc, i_in = to_canon(1000.0, (e1, 1, e2), ((-r, 0), (0, 0), (r, 0)))
            cells.append((e1, e2, k, tc))
past = [c for c in cells if c[3] > CAP]
print("   %d (eps1, eps2, k) cells; t = 1000 -> %.4g to %.4g; past the cap of 200: %d of %d"
      % (len(cells), min(c[3] for c in cells), max(c[3] for c in cells), len(past), len(cells)))
for e1 in eps:
    for e2 in eps:
        sub = [c for c in cells if c[0] == e1 and c[1] == e2]
        under = [c[2] for c in sub if c[3] <= CAP]
        print("   (eps1, eps2) = (%g, %g): t = 1000 -> %.4g (k = 0.05) to %.4g (k = 0.95); within the cap: %s"
              % (e1, e2, sub[0][3], sub[-1][3], "k <= %.2f" % max(under) if under else "no k"))

print("3. Mikkola & Hietarinta 1989: E = -1, unit masses, start q1 = q2 = R")
fac = []
for nu in range(1, 101):
    R = (nu - 0.5) * 0.025
    f, _ = to_canon(1.0, (1, 1, 1), ((-R, 0), (0, 0), (R, 0)))
    fac.append((R, f))
print("   one source time unit -> %.4g integrator units at R = %.4f, %.4g at R = %.4f"
      % (fac[0][1], fac[0][0], fac[-1][1], fac[-1][0]))
print("   the cap of 200 is reached at source time %.4g (R = %.4f) to %.4g (R = %.4f)"
      % (CAP / fac[0][1], fac[0][0], CAP / fac[-1][1], fac[-1][0]))
t3 = (("t1", 66.6), ("t2", 129.0), ("t3", 207.3), ("t4", 316.6), ("t5", 482.7), ("t6", 789.2), ("t7", 1745.0),
      ("t8", 9e7))
for name, t in t3:
    within = sum(1 for R, f in fac if t * f <= CAP)
    print("   Table 3 one-directional %s = %g: within the cap at %d of the 100 R values; %.4g to %.4g integrator units"
          % (name, t, within, t * fac[-1][1], t * fac[0][1]))

print("4. Martynova & Orlov 2014: T_cr per start, at rest, unit masses")
N = 1000
tcr = []
for i in range(1, N):
    for j in range(1, N + 1):
        if (i + N // 2) ** 2 + j * j < N * N:
            xi, eta = i / N, j / N
            pts = ((-0.5, 0.0), (0.5, 0.0), (xi, eta))
            u = -sum(1 / math.dist(pts[a], pts[b]) for a, b in ((0, 1), (0, 2), (1, 2)))
            t_cr_src = 3 ** 2.5 / (2 * abs(u)) ** 1.5
            tcr.append((to_canon(t_cr_src, (1, 1, 1), pts)[0], xi, eta))
n, lo, hi = len(tcr), min(tcr), max(tcr)
s3 = math.sqrt(3)
eq = (2 * (1 / (3 * s3))) ** -1.5
print("   %d interior points; T_cr in integrator units %.4g (at (%g, %g)) to %.4g (at (%g, %g))"
      % (n, lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]))
print("   the equilateral start at I = 1 (|E| = 1/(3 sqrt 3) = %.5f, on the arc, excluded): T_cr = %.4f" % (1 / (3 * s3), eq))
print("   30 T_cr: %.4g to %.4g; within the cap at every grid point: %s" % (30 * lo[0], 30 * hi[0], 30 * hi[0] <= CAP))
print("   10^5 T_cr: %.4g to %.4g; 10^6 T_cr: %.4g to %.4g; 10^7 T_cr: %.4g to %.4g"
      % (1e5 * lo[0], 1e5 * hi[0], 1e6 * lo[0], 1e6 * hi[0], 1e7 * lo[0], 1e7 * hi[0]))
print("   grid points with 10^5 T_cr within the cap: %d of %d" % (sum(1 for t in tcr if 1e5 * t[0] <= CAP), n))
print("   largest multiple of T_cr within the cap at every grid point: %.4g T_cr" % (CAP / hi[0]))
