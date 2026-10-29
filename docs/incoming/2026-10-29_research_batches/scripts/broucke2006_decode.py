"""broucke2006_decode.py <broucke_2006 dir>: decode Broucke, Elipe & Riaguas (2006) Table 1 (rotating Jacobian
variables) into barycentric inertial Cartesian initial conditions, and test the decoding (paper arithmetic, no
integration). Table 1 is read from <dir>/table1_reconciled.csv (columns y, xdot, T).

Frame (p. 515): X-axis along m1 -> m2, origin at their midpoint, rotating; m1 = (-r, 0), m2 = (r, 0),
m3 = (x, y); the state is x, y, r and their rotating-frame rates; masses 1/3 each (p. 515); zero angular
momentum. Table 1: x = ydot = rdot = 0, r = 1, given y, xdot, T. The frame's angular rate w is not printed:
it follows from C = 0. With masses m: inertial velocities m1 (0, -w r), m2 (0, w r), m3 (xdot - w y, 0);
barycentre (0, y/3); C = m [2 w r^2 - (2/3) y (xdot - w y)] = 0  =>  w = y xdot / (3 r^2 + y^2).
G is not stated in this paper (it refers to refs [7, 10]); the decoding takes G = 1 and tests it with the
scale invariant: for equal masses m and G = 1, T |E|^(3/2) / m^(5/2) is invariant
under rescaling, so it must equal that of a known figure-eight with m = 1, G = 1 (threebodyorbits V.1.A
T = 6.325913982926211, E = -1.287142; V.1.B T = 6.290750838848716, E = -1.291934).

The other readings tested after the main one (same arithmetic):
  (1) masses 1 instead of 1/3, G = 1;
  (2) xdot read as the inertial x-velocity of m3 (u = xdot instead of xdot - w y; then C = 0 gives
      w = y xdot / (3 r^2)), for masses 1/3 and 1, G = 1;
  (3) masses 1/3 and G free: for general G the invariant is T |E|^1.5 / (G m^2.5), with E = K + G U1 (U1 the
      potential at G = 1); it rises monotonically in G on E < 0, so the G that makes it equal the reference
      is found by bisection;
  (4) the virial ratio 2K/|U| at the isosceles instant, for G = 1, m = 1/3.

Run: python3 -I scripts/broucke2006_decode.py broucke_2006 > broucke_2006/decode_attempt.txt
"""
import csv, math, os, sys
d = sys.argv[1]
m = 1 / 3
NAMES = ["Table1 row1 (MCM figure-8, stable per Table 2)", "Table1 row2 (Simo H3, unstable per Table 2)"]
T1 = list(csv.DictReader(open(os.path.join(d, "table1_reconciled.csv"), encoding="utf-8")))
assert len(T1) == len(NAMES)
ROWS = [(NAMES[i], float(r["y"]), float(r["xdot"]), float(r["T"])) for i, r in enumerate(T1)]
REF = {"V.1.A": (6.325913982926211, -1.287142), "V.1.B": (6.290750838848716, -1.291934)}


def state(y, xd, r=1.0, inertial_xdot=False):
    """Barycentric positions and velocities (mass-independent) and the frame rate w."""
    w = y * xd / (3 * r * r) if inertial_xdot else y * xd / (3 * r * r + y * y)
    u = xd if inertial_xdot else xd - w * y
    q = [(-r, -y / 3), (r, -y / 3), (0.0, 2 * y / 3)]
    v = [(-u / 3, -w * r), (-u / 3, w * r), (2 * u / 3, 0.0)]
    return w, q, v


def energy(q, v, mass, G=1.0):
    K = 0.5 * mass * sum(a * a + b * b for a, b in v)
    U = -G * mass * mass * sum(1 / math.dist(q[i], q[j]) for i in range(3) for j in range(i + 1, 3))
    return K, U


for k, (T, E) in REF.items():
    print("reference %s: T |E|^1.5 = %.6f (m = 1, G = 1)" % (k, T * abs(E) ** 1.5))
REF_INV = REF["V.1.A"][0] * abs(REF["V.1.A"][1]) ** 1.5
for name, y, xd, T in ROWS:
    r = 1.0
    w, q, v = state(y, xd, r)
    K, U = energy(q, v, m)
    E = K + U
    C = m * sum(a[0] * b[1] - a[1] * b[0] for a, b in zip(q, v))
    P = (m * sum(a for a, _ in v), m * sum(b for _, b in v))
    inv = T * abs(E) ** 1.5 / m ** 2.5
    print("%s: w = %.10f; E = %.10f (G = 1); C = %.1e; P = (%.1e, %.1e); T |E|^1.5 / m^2.5 = %.6f" % (name, w, E, C, P[0], P[1], inv))
    # rescale to masses 1, G = 1, E = -1.287142 (the V.1.A energy) for a direct comparison of T
    s_m = 1 / m                         # mass factor
    # with G = 1: E ~ M^2 / L  ->  L factor = s_m^2 * E / E_target ; T ~ sqrt(L^3 / M)
    if E >= 0:
        print("    E >= 0 with G = 1 and masses 1/3: the orbit would be unbound, so this reading of the variables is wrong")
    for k, (Tr, Er) in (REF.items() if E < 0 else []):
        L = s_m ** 2 * E / Er
        Ts = T * math.sqrt(L ** 3 / s_m)
        print("    rescaled to m = 1, G = 1, E = %.6f: T = %.7f  (catalogue %s T = %.7f)" % (Er, Ts, k, Tr))
    print("    barycentric inertial state (m = 1/3, G = 1, as printed scale): q = %s; v = %s" % (
        [tuple(round(c, 10) for c in p) for p in q], [tuple(round(c, 10) for c in p) for p in v]))

print()
print("Other readings (reference T |E|^1.5 = %.4f, V.1.A):" % REF_INV)
for name, y, xd, T in ROWS:
    print(name)
    w, q, v = state(y, xd)
    K1, U1 = energy(q, v, 1.0)
    print("  (1) masses 1, G = 1: E = %.4f; T |E|^1.5 = %.3f" % (K1 + U1, T * abs(K1 + U1) ** 1.5))
    wi, qi, vi = state(y, xd, inertial_xdot=True)
    for mass, label in ((m, "1/3"), (1.0, "1")):
        Ki, Ui = energy(qi, vi, mass)
        print("  (2) xdot as m3's inertial x-velocity, masses %s, G = 1: w = %.6f; E = %.4f (%s)" % (
            label, wi, Ki + Ui, "unbound" if Ki + Ui >= 0 else "bound"))
    K, U = energy(q, v, m)
    f = lambda G: T * abs(K + G * U) ** 1.5 / (G * m ** 2.5)
    lo = hi = K / -U                    # E = 0 here; E < 0 above it, where f rises monotonically
    while f(hi) < REF_INV:
        hi *= 2
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        lo, hi = (mid, hi) if f(mid) < REF_INV else (lo, mid)
    G = 0.5 * (lo + hi)
    print("  (3) masses 1/3, G solved from T |E|^1.5 / (G m^2.5) = %.4f: G = %.3f (E = %.4f)" % (REF_INV, G, K + G * U))
    print("  (4) virial ratio 2K/|U| at the isosceles instant, G = 1, m = 1/3: %.2f" % (2 * K / -U))

print("""
Conclusion: no reading fits. With masses 1/3 and G = 1 both states are unbound; with masses 1 they are bound but
T |E|^1.5 is far from the figure-eight's; reading xdot as m3's inertial velocity leaves E > 0 for either mass
choice; the G that would match the reference differs between the rows, i.e. no single G; and at the isosceles
instant the virial ratio 2K/|U| should be below 1 (m3 at apoapsis), not above 4. The variables cannot be decoded
from this paper alone; the equations are in refs [7] (Broucke, Elipe & Riaguas, AIAA 2002-4544) and [10] (Broucke
& Boggs 1975, CM 11, 13-38), neither in the repo. Queued as RQ-641 (README "To apply"). Consistency that does
hold: Table 2's y/r equals Table 1's y to 1e-8 (same orbits, other scale), and Table 3's f1, f3 follow from a1, a2
as its footnote defines (checks.txt).""")
