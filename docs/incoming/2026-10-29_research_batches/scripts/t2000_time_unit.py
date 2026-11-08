"""t2000_time_unit.py: is a printed 'time' the instant of the n_min-th minimum in Paper I's units?

TASK-M9-43 (R-702, REQ-VAL-351). Paper I (Tanikawa, Umehara & Abe 1995, CeMDA 62, 335) p. 342 states
"In the units G = 1, M = 1, AB = 1"; M is not defined on its pages. This integrates the free-fall start
of each printed row in the frame both papers state (m2 at A(-0.5, 0), m3 at B(+0.5, 0), m1 at P(x, y),
all at rest, equal masses) with G = 1, each mass 1, AB = 1, and reports the time of the n_min-th
minimum of r = min(r12, r23, r13) (Paper I p. 339's r; Tanikawa 2000 p. 167's "minimal mutual
distances"). Each mass 1/3 (M = 1 as the total) multiplies every time by sqrt(3) exactly, since in
free fall t scales as m^(-1/2) at fixed lengths; that column is printed beside it.

Method: Sundman-type time transformation dt/ds = 1/U, U = sum 1/r_ij, the physical time carried in the
state; scipy DOP853, rtol 1e-12, atol 1e-14. Minima of r are the + direction zeros of dr/dt. When r
falls to R_BOUNCE: if the closest pair meets head-on (as m1 and m3 do on the circle Gamma_3, where
the triangle stays isosceles), that is a collision, counted as a minimum and, if it is not the
n_min-th, continued by reversing the pair's relative velocity about its centre of mass, the
regularised continuation of a head-on collision; otherwise the run integrates through the near-miss,
and a fall below R_STOP is taken as the collision itself, which ends the run (reported as the
n_min-th minimum if it is, else STOPPED). The rows of Tables I-IV, all on Gamma_3, start from the
printed alpha (radians, Table I's note), x = -0.5 + cos(alpha), y = sin(alpha), so the isosceles
symmetry is exact. The printed x, y do not place them as well: measured from the reconciled CSVs,
they differ from these by up to 9.2e-7 (52 of the 59 rows do not round to the printed 7 decimals)
and lie off Gamma_3 radially by up to 9.9e-7, while atan2(y, x + 0.5) agrees with the printed alpha
to 1.3e-7. A run that passes t = 3 * printed + 1 without its n_min-th minimum is NOT REACHED. Column
t1 is the time of the first minimum of r on the same start, a lower bound on every later one.
A check, not a target (R-649).

Rows: Paper I Table IIa (p. 348; x, y, Time as printed on the page image; the PDF's text layer reads
row 19's Time as 0.7891, the page prints 0.4891) and Tanikawa 2000's reconciled tables
(tanikawa_2000/reconciled/). Run: python3 -I scripts/t2000_time_unit.py > tanikawa_2000/time_unit_check.txt
"""
import csv
import multiprocessing
import os
import sys
import time as wall

import numpy as np
import scipy
from scipy.integrate import solve_ivp

R_STOP = 1e-9  # a near-miss closer than this is taken as the collision itself
R_BOUNCE = 1e-6  # a head-on approach this close is a collision continued by reversal
HEAD_ON = 1e-6  # |d x dv| / (|d| |dv|) below this: the pair meets head-on
WALL_LIMIT = 600.0  # seconds per row, a safety stop only; the t cap decides
PAIRS = ((0, 1), (1, 2), (0, 2))  # r12 -> type 3, r23 -> type 1, r13 -> type 2 (Paper I p. 339)
TYPE = (3, 1, 2)

# Paper I Table IIa, p. 348: "Positions of the binary collision points of type 1 for n_min = 2".
PAPER_I_IIA = """0.4200000 0.4369369 0.6547
0.4100000 0.4318979 0.6497
0.4050000 0.4291917 0.6470
0.4030000 0.4280645 0.6458
0.4000000 0.4263124 0.6439
0.3800000 0.4133726 0.6314
0.3600000 0.3983746 0.6182
0.3386657 0.3800000 0.6034
0.3183789 0.3600000 0.5886
0.3004792 0.3400000 0.5749
0.2845334 0.3200000 0.5622
0.2702483 0.3000000 0.5505
0.2574134 0.2800000 0.5395
0.2458712 0.2600000 0.5293
0.2355006 0.2400000 0.5198
0.2262059 0.2200000 0.5111
0.2179108 0.2000000 0.5030
0.2105530 0.1800000 0.4957
0.2040820 0.1600000 0.4891
0.1984563 0.1400000 0.4833
0.1969779 0.1342000 0.4817
0.1969281 0.1340000 0.4816
0.1969281 0.1200000 0.4816
0.1896108 0.1000000 0.4737
0.1863410 0.0800000 0.4701
0.1838152 0.0600000 0.4673
0.1820201 0.0400000 0.4653"""


def rmin(y):
    p = y[:6].reshape(3, 2)
    d = [np.hypot(*(p[i] - p[j])) for i, j in PAIRS]
    k = int(np.argmin(d))
    return k, d[k]


def rhs(_s, y):
    p = y[:6].reshape(3, 2)
    a = np.zeros((3, 2))
    u = 0.0
    for i, j in PAIRS:
        d = p[j] - p[i]
        r = np.hypot(*d)
        u += 1.0 / r
        a[i] += d / r**3
        a[j] -= d / r**3
    g = 1.0 / u
    return np.concatenate([g * y[6:12], g * a.ravel(), [g]])


def rdot(_s, y):
    p = y[:6].reshape(3, 2)
    v = y[6:12].reshape(3, 2)
    k, r = rmin(y)
    i, j = PAIRS[k]
    return float(np.dot(p[i] - p[j], v[i] - v[j]) / r)


rdot.direction = 1


def close(_s, y):
    return rmin(y)[1] - R_STOP


close.terminal = True
close.direction = -1


def near(_s, y):
    return rmin(y)[1] - R_BOUNCE


near.terminal = True
near.direction = -1


def head_on(y):
    p = y[:6].reshape(3, 2)
    v = y[6:12].reshape(3, 2)
    k, r = rmin(y)
    i, j = PAIRS[k]
    d, dv = p[i] - p[j], v[i] - v[j]
    return abs(d[0] * dv[1] - d[1] * dv[0]) / (r * np.hypot(*dv)) < HEAD_ON


def nth_minimum(x, y, nmin, t_cap):
    """(time, r, type, how): time and r of the n_min-th minimum of r, or what stopped the run."""
    st = np.array([x, y, -0.5, 0.0, 0.5, 0.0] + [0.0] * 6 + [0.0])
    s0, n, bounces, t0 = 0.0, 0, 0, wall.time()
    events = [rdot, near]
    while wall.time() - t0 < WALL_LIMIT:
        sol = solve_ivp(rhs, (s0, s0 + 0.5), st, method="DOP853", rtol=1e-12, atol=1e-14, events=events)
        note = " (%d bounces before)" % bounces if bounces else ""
        for ye in sol.y_events[0]:
            n += 1
            if n == nmin:
                k, r = rmin(ye)
                return ye[12], r, TYPE[k], "minimum" + note
        if sol.status == 1:
            ye = sol.y_events[1][0]
            s0, st = sol.t_events[1][0], ye.copy()
            k, r = rmin(ye)
            if events[1] is close:  # a near-miss reached R_STOP: the collision itself
                n += 1
                if n == nmin:
                    return ye[12], r, TYPE[k], "collision" + note
                return ye[12], r, TYPE[k], "STOPPED at a collision at minimum %d%s" % (n, note)
            if head_on(ye):  # reverse the pair's relative velocity about its centre of mass
                n += 1
                if n == nmin:
                    return ye[12], r, TYPE[k], "collision" + note
                i, j = PAIRS[k]
                v = st[6:12].reshape(3, 2)
                vcm = 0.5 * (v[i] + v[j])
                v[i], v[j] = 2 * vcm - v[i], 2 * vcm - v[j]
                bounces += 1
            else:  # a near-miss: integrate through it, watching for R_STOP
                events = [rdot, close]
            continue
        if sol.status != 0:
            return sol.y[12, -1], float("nan"), None, "STOPPED: integrator (%d minima)" % n
        st, s0 = sol.y[:, -1], sol.t[-1]
        if events[1] is close and rmin(st)[1] > R_BOUNCE:
            events = [rdot, near]
        if st[12] > t_cap:
            return st[12], float("nan"), None, "NOT REACHED by t = %.3f (%d minima)" % (st[12], n)
    return st[12], float("nan"), None, "STOPPED: wall limit (%d minima)" % n


def report(name, rows):
    print("== %s" % name)
    print("%-6s %-10s %-10s %-10s %-10s %-10s %-9s %-8s %-4s %-10s %s"
          % ("n_min", "x", "y", "printed", "t(m=1)", "t(m=1/3)", "rel.diff", "r", "type", "t1(m=1)", "how"))
    with multiprocessing.Pool(4) as pool:
        res = pool.starmap(nth_minimum, [(float(x), float(y), nmin, 3 * float(tp) + 1)
                                          for nmin, x, y, tp, _w in rows])
        first = pool.starmap(nth_minimum, [(float(x), float(y), 1, 3 * float(tp) + 1)
                                            for _n, x, y, tp, _w in rows])
    for (nmin, x, y, tp, want_type), (t, r, ty, how), f in zip(rows, res, first):
        rel = (t - float(tp)) / float(tp)
        flag = "" if want_type is None or ty is None or ty == want_type else " (type %s printed)" % want_type
        print("%-6d %-10.10s %-10.10s %-10s %-10.7f %-10.7f %+9.1e %-8.1e %-4s %-10.7f %s%s"
              % (nmin, x, y, tp, t, t * 3**0.5, rel, r, ty, f[0], how, flag))
        sys.stdout.flush()


def main():
    here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    rec = os.path.join(here, "tanikawa_2000", "reconciled")
    print("t2000_time_unit.py: G = 1, each mass 1, AB = 1; R_STOP %g, R_BOUNCE %g, HEAD_ON %g; "
          "t(m=1/3) = t(m=1) * sqrt(3); numpy %s, scipy %s" % (R_STOP, R_BOUNCE, HEAD_ON, np.__version__,
                                                               scipy.__version__))
    report("Paper I (1995) Table IIa, p. 348, type 1, n_min = 2",
           [(2, a, b, c, 1) for a, b, c in (ln.split() for ln in PAPER_I_IIA.splitlines())])
    tables = (("table_unnumbered_p162.csv", "Tanikawa 2000 unnumbered table, p. 162"),
              ("table_I.csv", "Tanikawa 2000 Table I, p. 167 (triple collision points on Gamma_3)"),
              ("table_II.csv", "Tanikawa 2000 Table II (triple collision points)"),
              ("table_III.csv", "Tanikawa 2000 Table III (triple collision points)"),
              ("table_IV.csv", "Tanikawa 2000 Table IV (triple collision points)"),
              ("table_V.csv", "Tanikawa 2000 Table V (type 1)"),
              ("table_VI.csv", "Tanikawa 2000 Table VI (type 2)"),
              ("table_VII.csv", "Tanikawa 2000 Table VII (type 3)"),
              ("table_VIII.csv", "Tanikawa 2000 Table VIII (type as printed)"))
    ON_GAMMA3 = ("table_I.csv", "table_II.csv", "table_III.csv", "table_IV.csv")
    fixed = {"table_V.csv": 1, "table_VI.csv": 2, "table_VII.csv": 3}
    for fn, name in tables:
        with open(os.path.join(rec, fn), newline="", encoding="utf-8") as f:
            rows = []
            for r in csv.DictReader(f):
                want = int(r["type"]) if "type" in r else fixed.get(fn)
                x, y = r["x"], r["y"]
                if fn in ON_GAMMA3:  # placed on the circle from the printed alpha (radians)
                    x = "%.15f" % (-0.5 + np.cos(float(r["alpha"])))
                    y = "%.15f" % np.sin(float(r["alpha"]))
                rows.append((int(r["n_min"]), x, y, r["time"], want))
        report(name, rows)


if __name__ == "__main__":
    main()
