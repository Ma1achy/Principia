"""The integrator's time unit for a source's ICs, so R-610's horizon cap of 200 can be read in the source's units.

Usage: python3 -I integrator_gauge_times.py dandekar <dandekar_2026/table1_reconciled.csv>
       python3 -I integrator_gauge_times.py melnikov

The integrator runs in one unit system, M = sum m_i = 1, G = 1, I = 1 (docs/contracts/principia_integrator_contract.md,
Part 5), I the moment of inertia sum m_i |r_i|^2 about the centre of mass at t = 0. A source in G = 1 units with total
mass M and initial I maps onto it by m' = m/M, r' = r/l with l = sqrt(I/M), and t' = t/t0 with t0 = sqrt(l^3/(G M)).
So one integrator time unit is t0 source units, and the cap T = 200 is 200 t0.

dandekar: Dandekar et al. 2026, §3.1: the lightest body is the single, the other two a circular binary with a = 5 AU,
the single and the binary's centre of mass at rest d = 100 AU apart. I = mu_B a^2 + mu_out d^2 (mu_B = ma mb/(ma + mb),
mu_out = (ma + mb) ms/M), independent of the binary phase. Source units: G = 1 with M_sun and AU, 1 yr = 2 pi
(dandekar2026_units.py). Printed per mass set: t0 in G = 1 units and in years, the cap in years, and the paper's
thresholds (eq. (30): 30 yr, 80 yr, 10 yr; Table 2's fit range 3000 yr to 1e6 yr) in integrator units.

melnikov: Mel'nikov, Orlov & Shevchenko 2013, eqs (2)-(3), G = 1, m1 = m2 = m3 = 1, phi = pi/2: m1 at (xA, -1), m2 at
(xA, 1), m3 at (xA - 2*6^(1/3) + dx, 0), xA = (16/9)^(1/3), dx from 0 to 1. Printed for dx = 0, 0.17, 0.2, 0.5, 1: t0,
the cap in the paper's units, and the fit range 2e3 <= T_d <= 1e5 in integrator units.
"""
import csv
import math
import sys


def t0_of(I, M):
    length = math.sqrt(I / M)
    return math.sqrt(length ** 3 / M)


def dandekar(path):
    yr = 2 * math.pi
    a, d = 5.0, 100.0
    print("Dandekar et al. 2026, the eight mass sets; t0 = one integrator time unit")
    for r in csv.DictReader(open(path, encoding="utf-8")):
        m = [float(x) for x in r["mass_set"].split(",")]
        s = min(range(3), key=lambda i: m[i])
        ma, mb = [m[i] for i in range(3) if i != s]
        ms, M = m[s], sum(m)
        I = ma * mb / (ma + mb) * a ** 2 + (ma + mb) * ms / M * d ** 2
        t0 = t0_of(I, M)
        ty = t0 / yr
        print(f"{r['mass_set']:>13}: I = {I:.4f} M_sun AU^2; t0 = {t0:.4f} G=1 units = {ty:.4f} yr; "
              f"cap 200 = {200 * ty:.1f} yr | 30 yr = {30 / ty:.2f}, 80 yr = {80 / ty:.2f}, 10 yr = {10 / ty:.2f}, "
              f"3000 yr = {3000 / ty:.1f}, 1e6 yr = {1e6 / ty:.4g} integrator units")


def melnikov():
    xA = (16 / 9) ** (1 / 3)
    s6 = 6 ** (1 / 3)
    print("Mel'nikov, Orlov & Shevchenko 2013, phi = pi/2; t0 = one integrator time unit, in the paper's units")
    for dx in (0.0, 0.17, 0.2, 0.5, 1.0):
        P = [(xA, -1.0), (xA, 1.0), (xA - 2 * s6 + dx, 0.0)]
        cx = sum(p[0] for p in P) / 3
        cy = sum(p[1] for p in P) / 3
        I = sum((p[0] - cx) ** 2 + (p[1] - cy) ** 2 for p in P)
        t0 = t0_of(I, 3.0)
        print(f"dx = {dx:4.2f}: I = {I:.4f}; t0 = {t0:.4f}; cap 200 = {200 * t0:.1f} paper units | "
              f"T_d = 2e3 is {2e3 / t0:.0f}, T_d = 1e5 is {1e5 / t0:.0f} integrator units")


if __name__ == "__main__":
    if sys.argv[1] == "dandekar":
        dandekar(sys.argv[2])
    elif sys.argv[1] == "melnikov":
        melnikov()
    else:
        sys.exit("usage: integrator_gauge_times.py dandekar <table1_reconciled.csv> | melnikov")
