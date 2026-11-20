"""Decode Titov (2024) chapter 5 (Tables 10 and 11): variables, regularised time, initial conditions.

Usage: python3 -I titov_ch5_decode.py <table10_reconciled.csv> <table11_reconciled.csv> <out.txt>

Nothing here is engine code; it re-derives the paper's own equations and integrates them to
check that the decoding reproduces the paper's tabulated periods.

Decoding (each step derived here with sympy, not copied from the printed formulas, several of
which carry typographical errors; see the README):
  * Units: m1 = m2 = m3 = 1, V = sum 1/r_ij (G = 1), energy h = -1 (p. 111, 126).
  * Size: rho (printed rho0, the paper's varrho) satisfies rho^2 = I = sum_i |r_i - r_cm|^2
    = (r12^2 + r13^2 + r23^2)/3, checked below from the paper's distance formulas (5.3), (5.23).
  * Collinear (5.1): body 3 between 1 and 2; shape angle phi with r12 = rho sqrt(1+cos phi),
    r13 = rho sqrt(1-cos(phi-pi/3)), r23 = rho sqrt(1-cos(phi+pi/3)) (from (5.3));
    regularised angle E by (5.8): cos phi = (5+cos4E)/(7-cos4E), sin phi = 4 sqrt3 sin2E/(7-cos4E).
    Collisions (r23 = 0 or r13 = 0) at E = pi/4 + k pi/2.
  * Isosceles (5.2): m1, m2 at the base; r12 = rho sqrt(1+cos th), r13 = r23 = rho sqrt(1-cos(th)/2)
    (5.23); E by (5.29): cos th = (cos2E-5)(1+3cos2E)/(3cos^2 2E+2cos2E+11),
    sin th = -8 sqrt3 sinE sin2E/(3cos^2 2E+2cos2E+11). Collision r12 = 0 at E = k pi.
  * Kinetic energy: computed here from Cartesian positions of unit masses on the line / in the
    isosceles triangle, then expressed in (rho, E); p_rho, p_E are the canonical momenta.
  * Regularised time: H' = u(q)(H - h), with u = 1 + cos4E (collinear, (5.15)) and
    u = 1 - cos2E (isosceles, (5.36)); dtau/dt = 1/u, so physical time t = integral of u dtau.
    The periods T in Tables 10 and 11 are in tau ("в единицах регуляризованного времени").
  * Initial conditions (pp. 111, 117, 125-128): on a collision ray, p_rho = 0 and p_E from H' = 0;
    rows marked * in Table 10 start on the Euler ray E = 0 with p_rho = 0, p_E from H' = 0;
    rows marked * in Table 11 are brake orbits starting at rest on the zero-velocity curve at
    angle E0 (p_rho = p_E = 0, rho0 from V = -h).
Each orbit is integrated (scipy DOP853, rtol = atol = 1e-12) for 1.5 T in tau; the reported
return time is the tau in [0.5 T, 1.5 T] minimising the distance in (rho, cos E, sin E, p_rho, p_E)
to the initial state, refined by a local minimiser. The physical period is integral of u dtau over
that interval.
"""
import csv
import math
import sys

import numpy as np
import sympy as sp
from scipy.integrate import solve_ivp
from scipy.optimize import minimize_scalar

rho, E, rhod, Ed, prho, pE = sp.symbols("rho E rhod Ed p_rho p_E", real=True)


def kinetic_collinear():
    # unit masses on a line, x1 < x3 < x2, barycentre 0; a = r13, b = r23
    phi = sp.atan2(4 * sp.sqrt(3) * sp.sin(2 * E), 5 + sp.cos(4 * E))
    a = rho * sp.sqrt(1 - sp.cos(phi - sp.pi / 3))
    b = rho * sp.sqrt(1 - sp.cos(phi + sp.pi / 3))
    x1, x3, x2 = -(2 * a + b) / 3, (a - b) / 3, (a + 2 * b) / 3
    return [x1, x2, x3], [(x1, 0), (x2, 0), (x3, 0)], a, b


def build(kind):
    t = sp.Symbol("t")
    if kind == "collinear":
        cphi = (5 + sp.cos(4 * E)) / (7 - sp.cos(4 * E))
        sphi = 4 * sp.sqrt(3) * sp.sin(2 * E) / (7 - sp.cos(4 * E))
        # r13, r23 from 1 - cos(phi -+ pi/3) = 1 - (cphi/2 +- sqrt3/2 sphi)
        r12 = rho * sp.sqrt(1 + cphi)
        r13 = rho * sp.sqrt(1 - (cphi / 2 + sp.sqrt(3) / 2 * sphi))
        r23 = rho * sp.sqrt(1 - (cphi / 2 - sp.sqrt(3) / 2 * sphi))
        V = 1 / r12 + 1 / r13 + 1 / r23
        u = 1 + sp.cos(4 * E)
        # positions: x1 < x3 < x2 on the x axis, barycentre 0
        a, b = r13, r23
        pos = [(-(2 * a + b) / 3, 0), ((a + 2 * b) / 3, 0), ((a - b) / 3, 0)]
    else:
        cth = (sp.cos(2 * E) - 5) * (1 + 3 * sp.cos(2 * E)) / (3 * sp.cos(2 * E) ** 2 + 2 * sp.cos(2 * E) + 11)
        r12 = rho * sp.sqrt(1 + cth)
        r13 = rho * sp.sqrt(1 - cth / 2)
        V = 1 / r12 + 2 / r13
        u = 1 - sp.cos(2 * E)
        hgt = sp.sqrt(r13 ** 2 - r12 ** 2 / 4)
        pos = [(-r12 / 2, -hgt / 3), (r12 / 2, -hgt / 3), (0, 2 * hgt / 3)]
    I = sum(px ** 2 + py ** 2 for px, py in pos)
    # kinetic energy from Cartesian velocities: d/dt of positions via chain rule
    T = 0
    for px, py in pos:
        vx = sp.diff(px, rho) * rhod + sp.diff(px, E) * Ed
        vy = sp.diff(py, rho) * rhod + sp.diff(py, E) * Ed
        T += (vx ** 2 + vy ** 2) / 2
    return dict(V=V, u=u, I=I, T=T, pos=pos, r12=r12)


def numeric_checks(kind, M, out):
    f_I = sp.lambdify((rho, E), M["I"], "mpmath")
    T = M["T"]
    a_rr = sp.lambdify((rho, E), sp.diff(T, rhod, 2), "math")
    a_EE = sp.lambdify((rho, E), sp.diff(T, Ed, 2), "math")
    a_rE = sp.lambdify((rho, E), sp.diff(T, rhod, Ed), "math")
    pts = [(1.3, 0.2), (2.1, 0.55), (0.7, 1.1), (3.0, 2.0), (1.0, 2.9)]
    for r_, e_ in pts:
        if kind == "collinear":
            c4 = math.cos(4 * e_)
            paper = {"(5.11) T = 1/2(rho'^2 + 24 rho^2 (1+cos4E)/(7-cos4E)^2 E'^2) -> 24 rho^2 (1+cos4E)/(7-cos4E)^2": 24 * r_**2 * (1 + c4) / (7 - c4) ** 2}
        else:
            c2 = math.cos(2 * e_)
            q = 3 * c2**2 + 2 * c2 + 11
            paper = {"(5.32) with (3+cos2E)^1: 24 rho^2 (1-cos2E)(3+cos2E)/q": 24 * r_**2 * (1 - c2) * (3 + c2) / q,
                     "(5.32) with (3+cos2E)^1 over q^2": 24 * r_**2 * (1 - c2) * (3 + c2) / q**2,
                     "(5.34) form, (1-cos2E)(3+cos2E)^2/q^2": 24 * r_**2 * (1 - c2) * (3 + c2) ** 2 / q**2}
        ours = a_EE(r_, e_)
        cmp = "; ".join(f"{k} = {v:.12f}" for k, v in paper.items())
        out.append(f"  {kind}: rho={r_} E={e_}: I/rho^2 = {float(f_I(r_, e_)) / r_**2:.15f}; "
                   f"d2T/drho'^2 = {a_rr(r_, e_):.12f} (paper 1); d2T/drho'dE' = {a_rE(r_, e_):.1e} (paper 0); "
                   f"d2T/dE'^2 ours = {ours:.12f}; {cmp}")
    return a_rr, a_EE, a_rE


def make_system(M, kind, out, h=-1.0):
    # T = 1/2 rhod^2 + 1/2 rho^2 G(E) Ed^2 (cross term checked to vanish); V = D(E)/rho.
    # At a collision u -> 0, G -> 0 and D -> infinity, so H' = u (H - h) is written with the
    # smooth products u/G and u*D. Their closed forms (those of the paper's (5.15) and (5.36))
    # are checked here against G and D derived from the Cartesian positions, away from collisions.
    G = sp.diff(M["T"], Ed, 2).subs(rho, 1)
    Dexpr = M["V"].subs(rho, 1)
    u = M["u"]
    if kind == "collinear":
        c4 = sp.cos(4 * E)
        uG = (7 - c4) ** 2 / 24
        uD = sp.sqrt(7 - c4) * (9 + c4) / (2 * sp.sqrt(3))
    else:
        c2 = sp.cos(2 * E)
        q = 3 * c2 ** 2 + 2 * c2 + 11
        uG = q ** 2 / (24 * (3 + c2) ** 2)
        uD = sp.sqrt(q) * (7 - 3 * c2) / (sp.sqrt(6) * (3 + c2))
    # the check runs in mpmath at 40 digits: in doubles the Cartesian-derived G loses digits to
    # cancellation in 1 - cos(...) near the collision rays
    import mpmath
    mpmath.mp.dps = 40
    fGc, fDc, fuc = sp.lambdify(E, G, "mpmath"), sp.lambdify(E, Dexpr, "mpmath"), sp.lambdify(E, u, "mpmath")
    fuG0, fuD0 = sp.lambdify(E, uG, "mpmath"), sp.lambdify(E, uD, "mpmath")
    worst = 0.0
    for e in map(mpmath.mpf, ("0.1", "0.33", "0.6", "1.0", "1.4", "2.2", "2.9", "4.0", "5.5")):
        r1 = abs(fuc(e) / fGc(e) / fuG0(e) - 1)
        r2 = abs(fuc(e) * fDc(e) / fuD0(e) - 1)
        worst = max(worst, r1, r2)
    out.append(f"  {kind}: closed forms u/G = {uG}, u*D = {uD}; largest relative difference from the "
               f"Cartesian-derived values at 9 test angles (40 digits): {mpmath.nstr(worst, 2)}")
    fDc = sp.lambdify(E, Dexpr, "math")
    f_uG, f_uGp = sp.lambdify(E, uG, "math"), sp.lambdify(E, sp.diff(uG, E), "math")
    f_uD, f_uDp = sp.lambdify(E, uD, "math"), sp.lambdify(E, sp.diff(uD, E), "math")
    fu, fup = sp.lambdify(E, u, "math"), sp.lambdify(E, sp.diff(u, E), "math")

    def fH(r, e, pr, pe):
        return fu(e) * (pr * pr / 2 - h) + f_uG(e) * pe * pe / (2 * r * r) - f_uD(e) / r

    def fs(r, e, pr, pe):
        U = fu(e)
        dr = U * pr
        de = f_uG(e) * pe / (r * r)
        dpr = f_uG(e) * pe * pe / r ** 3 - f_uD(e) / (r * r)
        dpe = -(fup(e) * (pr * pr / 2 - h) + f_uGp(e) * pe * pe / (2 * r * r) - f_uDp(e) / r)
        return [dr, de, dpr, dpe]

    def fV(r, e):
        return fDc(e) / r
    return fs, fH, fu, fV, None


def pE_from_energy(fH, r0, E0):
    # H' is quadratic in p_E: H'(p_E) = H'(0) + c p_E^2
    h0 = fH(r0, E0, 0.0, 0.0)
    h1 = fH(r0, E0, 0.0, 1.0)
    c = h1 - h0
    return math.sqrt(-h0 / c)


def run(kind, row, sysfun, out):
    fs, fH, fu, fV, fg = sysfun
    T_tab = float(row["T"])
    v = float(row["rho0_or_E0"])
    star = row["star"] == "*"
    if kind == "collinear":
        if star:
            r0, E0 = v, 0.0
        else:
            r0, E0 = v, math.pi / 4
        if abs(math.cos(4 * E0) + 1) < 1e-12:
            # on the collision ray H' = c p_E^2 + const exactly; same quadratic solve
            pass
        p0 = pE_from_energy(fH, r0, E0)
        y0 = [r0, E0, 0.0, p0]
    else:
        if star:
            E0 = v
            # brake orbit: rest on the zero-velocity curve, V(rho, E0) = 1 (h = -1): rho0 = rho*V(rho)
            r0 = fV(1.0, E0)  # V scales as 1/rho, so V(rho0) = 1 gives rho0 = V(1)
            y0 = [r0, E0, 0.0, 0.0]
        else:
            r0, E0 = v, 0.0
            y0 = [r0, E0, 0.0, pE_from_energy(fH, r0, E0)]

    def rhs(tau, y):
        d = fs(*y[:4])
        return d + [fu(y[1])]

    sol = solve_ivp(rhs, (0, 1.5 * T_tab), y0 + [0.0], method="DOP853", rtol=1e-12, atol=1e-12,
                    dense_output=True)

    def dist(tau):
        y = sol.sol(tau)
        return math.sqrt((y[0] - y0[0]) ** 2 + (math.cos(y[1]) - math.cos(y0[1])) ** 2
                         + (math.sin(y[1]) - math.sin(y0[1])) ** 2 + (y[2] - y0[2]) ** 2 + (y[3] - y0[3]) ** 2)
    grid = np.linspace(0.5 * T_tab, 1.5 * T_tab, 4001)
    dv = np.array([dist(x) for x in grid])
    k = int(dv.argmin())
    lo, hi = grid[max(k - 2, 0)], grid[min(k + 2, len(grid) - 1)]
    res = minimize_scalar(dist, bounds=(lo, hi), method="bounded", options={"xatol": 1e-12})
    tau_ret = res.x
    t_phys = sol.sol(tau_ret)[4]
    Hdrift = max(abs(fH(*sol.sol(x)[:4])) for x in np.linspace(0, tau_ret, 501))
    out.append(f"  {kind} row {row['row']}{' *' if star else ''}: start rho0={y0[0]:.6f} E0={y0[1]:.6f} "
               f"p_rho0=0 p_E0={y0[3]:.6f}; tabulated T(tau)={row['T']}; return tau={tau_ret:.6f} "
               f"(distance {res.fun:.1e}); diff={tau_ret - T_tab:+.2e}; physical period t={t_phys:.6f}; "
               f"max|H'| on orbit {Hdrift:.1e}")
    return tau_ret, res.fun, t_phys


def main(t10, t11, outp):
    out = ["Titov (2024) chapter 5 decoding check (h = -1, unit masses, G = 1)", ""]
    for kind, path in (("collinear", t10), ("isosceles", t11)):
        M = build(kind)
        out.append(f"{kind}: kinetic-energy check (paper (5.2)/(5.22): T = 1/2 (rho'^2 + 1/4 rho^2 angle'^2))")
        numeric_checks(kind, M, out)
        sysfun = make_system(M, kind, out)
        out.append(f"{kind}: integration of the regularised equations from the tabulated starts")
        for row in csv.DictReader(open(path)):
            run(kind, row, sysfun, out)
        out.append("")
    open(outp, "w").write("\n".join(out) + "\n")
    print("\n".join(out))


if __name__ == "__main__":
    main(*sys.argv[1:4])
