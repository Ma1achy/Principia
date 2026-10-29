"""broucke2006_decode.py: decode Broucke, Elipe & Riaguas (2006) Table 1 (rotating Jacobian variables) into
barycentric inertial Cartesian initial conditions, and test the decoding (paper arithmetic, no integration).

Frame (p. 515): X-axis along m1 -> m2, origin at their midpoint, rotating; m1 = (-r, 0), m2 = (r, 0),
m3 = (x, y); the state is x, y, r and their rotating-frame rates; masses 1/3 each (p. 515); zero angular
momentum. Table 1: x = ydot = rdot = 0, r = 1, given y, xdot, T. The frame's angular rate w is not printed:
it follows from C = 0. With masses m: inertial velocities m1 (0, -w r), m2 (0, w r), m3 (xdot - w y, 0);
barycentre (0, y/3); C = m [2 w r^2 - (2/3) y (xdot - w y)] = 0  =>  w = y xdot / (3 r^2 + y^2).
G is not stated in this paper (it refers to refs [7, 10]); the decoding takes G = 1 and tests it with the
scale invariant: for equal masses m and G = 1, T |E|^(3/2) / m^(5/2) is invariant
under rescaling, so it must equal that of a known figure-eight with m = 1, G = 1 (threebodyorbits V.1.A
T = 6.325913982926211, E = -1.287142; V.1.B T = 6.290750838848716, E = -1.291934).
"""
import math
m = 1 / 3
ROWS = [("Table1 row1 (MCM figure-8, stable per Table 2)", 2.3482448860, 2.8082814798, 19.0947186),
        ("Table1 row2 (Simo H3, unstable per Table 2)", 2.4538034560, 2.9258077218, 20.4002062)]
REF = {"V.1.A": (6.325913982926211, -1.287142), "V.1.B": (6.290750838848716, -1.291934)}
for k, (T, E) in REF.items():
    print("reference %s: T |E|^1.5 = %.6f (m = 1, G = 1)" % (k, T * abs(E) ** 1.5))
for name, y, xd, T in ROWS:
    r = 1.0
    w = y * xd / (3 * r * r + y * y)
    q = [(-r, -y / 3), (r, -y / 3), (0.0, 2 * y / 3)]
    v = [(-(xd - w * y) / 3, -w * r), (-(xd - w * y) / 3, w * r), (2 * (xd - w * y) / 3, 0.0)]
    K = 0.5 * m * sum(a * a + b * b for a, b in v)
    U = -m * m * sum(1 / math.dist(q[i], q[j]) for i in range(3) for j in range(i + 1, 3))
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
