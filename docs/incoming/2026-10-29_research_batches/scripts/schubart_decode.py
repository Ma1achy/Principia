"""schubart_decode.py <table3_reconciled.csv>: decode and check Schubart (1956) Table 3 and the p. 21-22 relations.

Paper-only arithmetic (no integration of the equations of motion, no engine code). Definitions, quoted from
pp. 21-22: m1 = m2 = m3 = 1, G = 1; t = 0 is the symmetric collision of m1 and m2; length unit r13 = r23 at
that collision; u = 2 * integral dt / (r12 r23); xi = r12 / 2; eta = r23 + r12 / 2; r13 = r12 + r23;
x2 = (r12 - r23) / 3; x2 = 0 at t = 0.21470; period = 4 * 0.21470; r13 max = 1.22949 there;
lim_{t->0} (4 / r12 - rdot12^2) = 4.91610.
"""
import csv, math, sys

R = [{k: float(v) for k, v in r.items()} for r in csv.DictReader(open(sys.argv[1]))]
TS, R13MAX, LIM = 0.21470, 1.22949, 4.91610

print("1. Identities per row (5-decimal table, so |residual| <= 1e-5 expected from rounding):")
w1 = w2 = 0.0
for r in R:
    a = r["r13"] - (r["xi"] + r["eta"])
    b = r["x2"] - (r["xi"] - r["eta"] / 3)
    w1, w2 = max(w1, abs(a)), max(w2, abs(b))
    if abs(a) > 1.01e-5 or abs(b) > 1.01e-5:
        print("   row u=%.1f: r13 - (xi+eta) = %.2e, x2 - (xi - eta/3) = %.2e" % (r["u"], a, b))
print("   max |r13 - (xi + eta)| = %.1e ; max |x2 - (xi - eta/3)| = %.1e over %d rows" % (w1, w2, len(R)))

print("2. Time convention: dt/du = r12 r23 / 2 = xi (eta - xi); Simpson's rule on the tabulated u grid (h = 0.1):")
f = [r["xi"] * (r["eta"] - r["xi"]) for r in R]
worst = 0.0
for n in range(2, len(R), 2):
    s = 0.1 / 3 * (f[0] + f[n] + 4 * sum(f[1:n:2]) + 2 * sum(f[2:n - 1:2]))
    d = s - R[n]["t"]; worst = max(worst, abs(d))
    print("   u=%.1f  t_table=%.5f  t_simpson=%.5f  diff=%+.1e" % (R[n]["u"], R[n]["t"], s, d))
print("   max |diff| = %.1e" % worst)

print("3. Turning point from the last two rows (x2 odd, r13 even about t*):")
r1, r2 = R[-2], R[-1]
tl = r1["t"] + (r2["t"] - r1["t"]) * (-r1["x2"]) / (r2["x2"] - r1["x2"])
print("   linear zero of x2 between u=2.1 and u=2.2: t = %.5f (text: %.5f)" % (tl, TS))
for r in (r1, r2):
    dt = r["t"] - TS
    print("   dt=%+.5f: (r13max - r13)/dt^2 = %.3f ; x2/dt = %.4f" % (dt, (R13MAX - r["r13"]) / dt ** 2, r["x2"] / dt))

print("4. Energy: E = rdot12^2/4 + etadot^2/3 - 1/r12 - 1/r23 - 1/r13 (Jacobi reduced masses 1/2 and 2/3);")
E = -LIM / 4 - 2.0
print("   at t -> 0: etadot = 0, r23 = r13 = 1, so E = -lim/4 - 2 = %.6f" % E)

print("5. Cartesian state at x2 = 0 (t = t*), barycentric, line = x-axis, order m1, m2, m3:")
h = R13MAX / 2
U = 2 / h + 1 / R13MAX
v1 = math.sqrt((U + E) / 3)          # v1 = v3, v2 = -2 v1 (symmetry and zero momentum); T = 3 v1^2
print("   q = (-%.6f, 0, +%.6f); U = %.6f; v1 = v3 = %.6f, v2 = %.6f (signs: m2 moves toward m3, x2 increasing)" % (h, h, U, -v1, 2 * v1))
print("   check: v2 from the table slope x2/dt above (~1.057) vs energy-derived 2 v1 = %.5f" % (2 * v1))
print("   period T = 4 t* = %.5f" % (4 * TS))

print("6. The same orbit rescaled to E = -1 (r -> L r, v -> v / sqrt(L), t -> L^1.5 t, L = |E|):")
L = -E
print("   L = %.6f; q3 = %.6f; v2 = %.6f; T = %.5f; collision-time r13 = %.6f" % (L, h * L, 2 * v1 / math.sqrt(L), 4 * TS * L ** 1.5, L))
