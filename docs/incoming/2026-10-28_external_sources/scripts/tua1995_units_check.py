"""Decode the mass unit of Tanikawa, Umehara & Abe (1995): "In the units G = 1, M = 1, AB = 1" (p. 342).

Usage: python3 -I tua1995_units_check.py <out.txt> x y time pair [x y time pair ...]

M could be the total mass (each body 1/3) or each body's mass (1). For each tabulated binary-collision point
(x, y, Time, pair) this integrates the free-fall problem (m2 at (-0.5, 0), m3 at (+0.5, 0), m1 at (x, y), all at
rest; equal masses m; G = 1; no regularisation) with scipy DOP853 (rtol = atol = 1e-12) for m = 1/3 and for m = 1,
until the integrator can no longer step (a collision) or 2 x the tabulated Time (x sqrt 3 for m = 1/3), and reports
where it stopped: the time, the closest pair and its distance. Not engine code; a decoding check only.
Times scale as m^(-1/2), so the two conventions differ by a factor sqrt(3) = 1.732 in every collision time.
"""
import math
import sys

import numpy as np
from scipy.integrate import solve_ivp


def rhs(m):
    def f(t, s):
        p = s[:6].reshape(3, 2)
        a = np.zeros((3, 2))
        for i in range(3):
            for j in range(3):
                if i != j:
                    d = p[j] - p[i]
                    a[i] += m * d / np.linalg.norm(d) ** 3
        return np.concatenate([s[6:], a.ravel()])
    return f


def stop(x, y, m, tmax):
    s0 = np.array([x, y, -0.5, 0.0, 0.5, 0.0] + [0.0] * 6)
    sol = solve_ivp(rhs(m), (0, tmax), s0, method="DOP853", rtol=1e-12, atol=1e-12)
    p = sol.y[:6, -1].reshape(3, 2)
    d = {(i + 1, j + 1): float(np.hypot(*(p[i] - p[j]))) for i, j in ((0, 1), (0, 2), (1, 2))}
    pr = min(d, key=d.get)
    return sol.t[-1], pr, d[pr], sol.status


def main(outp, args):
    lines = ["Tanikawa, Umehara & Abe (1995): which mass unit reproduces the tabulated collision times", ""]
    for i in range(0, len(args), 4):
        x, y, T, pair = float(args[i]), float(args[i + 1]), float(args[i + 2]), args[i + 3]
        lines.append(f"point ({x}, {y}): tabulated Time {T}, stated pair {pair}")
        for label, m in (("m = 1/3 (M = total mass 1)", 1 / 3), ("m = 1 (M = each mass 1)", 1.0)):
            t, pr, r, st = stop(x, y, m, 2 * T * (math.sqrt(3) if m < 1 else 1.0))
            how = "integrator stopped (collision)" if st == -1 else "reached the end time"
            lines.append(f"  {label}: {how} at t = {t:.6f}; closest pair {pr[0]}-{pr[1]}, distance {r:.1e}")
        lines.append("")
    open(outp, "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])
