"""zc_decode.py: Zare & Chesley 1998's coordinates, checked on their own printed numbers. Our arithmetic and our
integration of THEIR regularised equations (6), no Principia code. Run: python3 -I zc_decode.py > decode_check.txt
Units (p. 478): G = 1, 2(m1 + 4 m3) = 1, -8h = 1; equal masses (p. 479), so m = 0.1, alpha = m3/M = 1/3, h = -1/8.
"""
import math
import numpy as np
from scipy.integrate import solve_ivp

G, m, h, alpha = 1.0, 0.1, -0.125, 1.0 / 3.0
TOL = dict(method="DOP853", rtol=1e-13, atol=1e-14)

def p2_of(Q1, P1):
    """Eq. (11) at Q2 = 0, P2 >= 0: reduces to sqrt(alpha)/(2 Q1) * sqrt(1 - Q1^2 - P1^2) in these units."""
    return math.sqrt(alpha) / (2 * Q1) * math.sqrt(8 * h * Q1**2 + 2 * G * m + 8 * G * m - P1**2)

def rhs(_tau, s):
    """Eq. (6): Gamma's derivatives, with P0 = -h. State Q1, P1, Q2, P2."""
    Q1, P1, Q2, P2 = s
    R3 = (Q1**4 + Q2**2) ** 1.5
    return [P1 / 4,
            -(P2**2 * Q1 / alpha - G * m * 2 * Q1 * Q2**2 / R3 + 2 * (-h) * Q1),
            P2 * Q1**2 / alpha,
            -G * m * Q1**2 * Q2 / R3]

def fmap(Q1, P1, tau_max=400.0):
    """f: (Q1, P1) at one syzygy to the next, folded to Q1 > 0; k = zero crossings of Q1 on the way."""
    ev = lambda _t, s: s[2]
    ev.terminal, ev.direction = True, 0
    # Step off the section first so the start is not itself the event.
    s0 = [Q1, P1, 0.0, p2_of(Q1, P1)]
    off = solve_ivp(rhs, (0, 1e-3), s0, **TOL)
    sol = solve_ivp(rhs, (1e-3, tau_max), off.y[:, -1], events=ev, dense_output=False, max_step=0.05, **TOL)
    if not sol.t_events[0].size:
        return None
    y = sol.y_events[0][0]
    k = int(np.sum(np.diff(np.sign(sol.y[0])) != 0))
    return (abs(y[0]), P_fold(y), k)

def P_fold(y):
    return y[1] if y[0] > 0 else -y[1]

print("1. The section's domain (p. 478): 8hQ1^2 + 2Gm1 + 8Gm3 - P1^2 at Q2 = 0, against 1 - Q1^2 - P1^2:")
print("   ", 8 * h + 2 * G * m + 8 * G * m - 0.0, "at (Q1, P1) = (1, 0), expected 0")
n = sum(1 for a in range(1, 201) for b in range(-200, 201) if (a * .005) ** 2 + (b * .005) ** 2 < 1)
print(f"2. Lattice (p. 479): (m d, n d), d = 0.005, Q1 > 0, Q1^2 + P1^2 < 1: {n} points; printed 62,605")

print("3. Decode to Cartesian, from eqs (2), (5), W = p1 Q1^2 + p2 Q2 (p. 478) and p2 = alpha v (p. 477):")
print("   x = Q1^2, y = Q2, u = P1/(2 Q1), v = P2/alpha; m1 at (-x, -y/3), m2 at (x, -y/3), m3 at (0, 2y/3)")
Q1, P1 = 0.6, 0.2
Q2, P2 = 0.3, 0.0
P2 = math.sqrt(alpha) / (2 * Q1) * math.sqrt(8 * h * Q1**2 + 2 * m + 8 * m * Q1**2 / math.hypot(Q1**2, Q2) - P1**2)
x, y, u, v = Q1**2, Q2, P1 / (2 * Q1), P2 / alpha
r = np.array([[-x, -y / 3], [x, -y / 3], [0, 2 * y / 3]])
w = np.array([[-u, -v / 3], [u, -v / 3], [0, 2 * v / 3]])
T = .5 * m * (w**2).sum()
V = -sum(m * m / np.linalg.norm(r[i] - r[j]) for i, j in ((0, 1), (0, 2), (1, 2)))
print(f"   at a test point: Cartesian E = {T + V:.15f}; 2 m1 h = {2 * m * h:.15f}")
d = r[2] - r[1]
acc_m2 = G * m * (r[0] - r[1]) / np.linalg.norm(r[0] - r[1]) ** 3 + G * m * d / np.linalg.norm(d) ** 3
acc_m3 = sum(G * m * (r[i] - r[2]) / np.linalg.norm(r[i] - r[2]) ** 3 for i in (0, 1))
F3 = (x * x + y * y) ** -1.5
print(f"   du/dt: Newton {acc_m2[0]:.15f}; eq. (2) {-G * m / (4 * x * x) - G * m * x * F3:.15f}")
print(f"   dv/dt: Newton {acc_m3[1] - acc_m2[1]:.15f}; eq. (2) {-G * (2 * m + m) * y * F3:.15f}")
print("   Their units to unit masses, G = 1, E = -1: mass x 10, length x 2.5, time x 1.25, velocity x 2,")
print(f"   since E' = E M^2/L = {2 * m * h * 100 / 2.5:.6f} and tau = sqrt(L^3/M) = {math.sqrt(2.5**3 / 10):.6f}")
print("   Angular momentum vanishes on the set by symmetry (p. 476: 'zero angular momentum').")

print("4. Table II against their equations (our integration, DOP853 rtol 1e-13, of eq. (6); a decode check only):")
print("   k  q_k printed  k counted  f(q_k,0)-(q_k,0)        a_k ours    a_k printed  b_k ours   b_k printed")
TAB = [(1, .76297, -.9882, -.0095), (2, .53571, -8.65, 6.28), (3, .47891, -21.6, 15.7),
       (4, .45226, -37.2, 26.5), (5, .43611, -55.0, 38.4), (6, .42502, -74.9, 51.4)]
for k, q, a, b in TAB:
    f0 = fmap(q, 0.0)
    e = 1e-6
    # Df's entries a = dQ1'/dQ1, b = dQ1'/dP1, as Df's first row (p. 487), by central differences.
    fq1, fq2 = fmap(q + e, 0.0), fmap(q - e, 0.0)
    fp1, fp2 = fmap(q, e), fmap(q, -e)
    aa = (fq1[0] - fq2[0]) / (2 * e)
    bb = (fp1[0] - fp2[0]) / (2 * e)
    print(f"   {k}  {q:.5f}      {f0[2]}          ({f0[0] - q:+.2e}, {f0[1]:+.2e})   {aa:10.4f}  {a:10.4f}  {bb:10.4f}  {b:10.4f}")

print("5. Arithmetic on the printed numbers (recorded, not corrected):")
a1, b1 = -.9882, -.0095
print(f"   e from a1, b1 (p. 487): {math.sqrt(1 - b1**2 / (1 - a1**2)):.4f}; printed 0.998...")
print(f"   rotation number from a1: arccos(a1)/(2 pi) = {math.acos(a1) / (2 * math.pi):.5f}; printed 0.477...")
print(f"   a = cos(2 pi 0.477) = {math.cos(2 * math.pi * .477):.4f}; printed a1 = -0.9882")
T1 = {"inf": .199, 0: .151, 1: .527, 2: .0368, 3: .0168, 4: .0102, 5: .00703, 6: .00516}
s = sum(T1.values())
C, beta = .1017, 1.650
tail = sum(C * j**-beta for j in range(7, 2000001)) + C * 2000000.5 ** (1 - beta) / (beta - 1)
print(f"   Table I sum {s:.5f}; the fit's tail over k >= 7 {tail:.5f}; total {s + tail:.5f} (should be 1 less the zero-measure sets)")
for j in range(1, 7):
    print(f"   fit C k^-beta at k = {j}: {C * j**-beta:.5f}; Table I {T1[j]}")
print(f"   A_1 less R1 ... R1 = A1 n f(A1) (p. 488): mu(R_1) = 0.2293 <= mu(A_1) = 0.527, as it must be")
