"""Janković, Dmitrašinović & Šuvakov 2020 (CPC 250, 107052), Tables 3-6: our own conversion and overlap audit.

Usage: python3 -I jankovic_overlap.py <reconciled.csv> <belgrade_gallery.csv> <threebodyorbits_catalogue.json>
                                      <out_cartesian.csv> <out_overlap.csv>
Conventions, from the paper (§3.2-§3.3, eq. 5): m = 1, G = 1; Jacobi vectors rho = (r1 - r2)/sqrt2 = (a, 0),
lambda = (r1 + r2 - 2 r3)/sqrt6 = (b, 0) with b = 1; rho-dot = (0, c), lambda-dot = (0, d), d = L - a c; centre of mass
and its velocity zero. Hence r1 = (a/sqrt2 + b/sqrt6, 0), r2 = (-a/sqrt2 + b/sqrt6, 0), r3 = (-2b/sqrt6, 0),
v1 = (0, c/sqrt2 + d/sqrt6), v2 = (0, -c/sqrt2 + d/sqrt6), v3 = (0, -2d/sqrt6). Arithmetic: mpmath at 50 digits.
E and L are recomputed from the Cartesian state (not from eq. 5), and eq. 5 is evaluated separately as a check.

Matching rules (stated in the README):
 1. Belgrade gallery (BG26, kind 'ssol', 57 BHH satellites): a row matches when the gallery's printed a and c equal
    the table's as decimal numbers (exactly) and L agrees to the gallery's printed digits.
 2. threebodyorbits.com snapshot (OC26, 3,943 orbits): every equal-mass (1, 1, 1) row with E < 0 is compared by the
    scale-invariant pair (|L| |E|^(1/2), T |E|^(3/2)); a candidate is a row within relative tolerance tol in both.
    Counts are reported at tol = 1e-6, 1e-4 and 1e-3. As a looser screen, |L| |E|^(1/2) alone is also compared at 1e-4,
    in case a row's catalogued T were an absolute rather than a relative period. T in both is the relative period (the paper's Table 3 caption
    and §3.1; the atlas's T for its 'bhhsat' rows is the satellite's period at E = -1).
"""
import csv
import json
import sys
from decimal import Decimal

import mpmath as mp

mp.mp.dps = 50


def state(a, c, L, b=1):
    s2, s6 = mp.sqrt(2), mp.sqrt(6)
    d = L - a * c
    r = [(a / s2 + b / s6, 0), (-a / s2 + b / s6, 0), (-2 * b / s6, 0)]
    v = [(0, c / s2 + d / s6), (0, -c / s2 + d / s6), (0, -2 * d / s6)]
    return d, r, v


def energy_L(r, v):
    T = sum((vx * vx + vy * vy) for vx, vy in v) / 2
    U = 0
    for i in range(3):
        for j in range(i + 1, 3):
            U += 1 / mp.sqrt((r[i][0] - r[j][0]) ** 2 + (r[i][1] - r[j][1]) ** 2)
    Lz = sum(x * vy - y * vx for (x, y), (vx, vy) in zip(r, v))
    return T - U, Lz


def eq5(a, c, L):
    s3 = mp.sqrt(3)
    d = L - a * c
    return (c * c + d * d) / 2 - (mp.sqrt(2) / (a + s3) + mp.sqrt(2) / abs(a - s3) + 1 / (mp.sqrt(2) * a))


def main():
    rec, bg, oc, out_cart, out_ov = sys.argv[1:6]
    rows = list(csv.DictReader(open(rec, encoding="utf-8")))
    gal = [g for g in csv.DictReader(open(bg, encoding="utf-8")) if g["kind"] == "ssol" and g["id"] != ""]
    cat = json.load(open(oc, encoding="utf-8"))["orbits"]
    eqm = [o for o in cat if o["masses"] == [1.0, 1.0, 1.0] and o["E"] < 0]
    print(f"Belgrade gallery satellites: {len(gal)}; atlas rows: {len(cat)}, equal-mass with E < 0: {len(eqm)}")
    cart = csv.writer(open(out_cart, "w", newline="", encoding="utf-8"), lineterminator="\n")
    cart.writerow(["nr", "x1", "x2", "x3", "vy1", "vy2", "vy3", "d", "E", "L_recomputed", "eq5_minus_E",
                   "L_scaled", "T_scaled"])
    ov = csv.writer(open(out_ov, "w", newline="", encoding="utf-8"), lineterminator="\n")
    ov.writerow(["nr", "belgrade_gallery_match", "atlas_candidates_tol_1e-3", "atlas_best_rel_dL", "atlas_best_rel_dT",
                 "atlas_best_key", "atlas_best_rel_dL_alone"])
    n_bg = 0
    n_at = {1e-6: 0, 1e-4: 0, 1e-3: 0}
    worst_eq5 = worst_L = 0
    not_bg = []
    n_lonly = 0
    for r in rows:
        a, c, L, T = (mp.mpf(r[k]) for k in ("a", "c", "L", "T"))
        d, pos, vel = state(a, c, L)
        E, Lz = energy_L(pos, vel)
        worst_eq5 = max(worst_eq5, abs(eq5(a, c, L) - E))
        worst_L = max(worst_L, abs(Lz - L))
        Ls, Ts = abs(L) * mp.sqrt(-E), T * (-E) ** mp.mpf(1.5)
        cart.writerow([r["nr"]] + [mp.nstr(x, 25) for x in (pos[0][0], pos[1][0], pos[2][0], vel[0][1], vel[1][1],
                                                            vel[2][1], d, E, Lz, eq5(a, c, L) - E, Ls, Ts)])
        m = [g["id"] for g in gal
             if Decimal(g["rel_coord_a"]) == Decimal(r["a"]) and Decimal(g["rel_vel_c"]) == Decimal(r["c"])
             and abs(Decimal(g["angular_momentum"]) - Decimal(r["L"])) <= Decimal("5e-11")]
        if m:
            n_bg += 1
        else:
            not_bg.append(r["nr"])
        best = None
        cands = []
        lonly = []
        for o in eqm:
            e = abs(o["E"])
            oL, oT = abs(o["L"]) * e ** 0.5, o["T"] * e ** 1.5
            dl, dt = abs(oL - float(Ls)) / float(Ls), abs(oT - float(Ts)) / float(Ts)
            if best is None or max(dl, dt) < max(best[0], best[1]):
                best = (dl, dt, o["key"])
            lonly.append(dl)
            if dl <= 1e-3 and dt <= 1e-3:
                cands.append((dl, dt, o["key"]))
        for tol in n_at:
            if any(x[0] <= tol and x[1] <= tol for x in cands):
                n_at[tol] += 1
        ov.writerow([r["nr"], ";".join("ssol id " + i for i in m), ";".join(k for _, _, k in cands),
                     f"{best[0]:.3e}", f"{best[1]:.3e}", best[2], f"{min(lonly):.3e}"])
        if min(lonly) <= 1e-4:
            n_lonly += 1
    print(f"max |eq.5 E - Cartesian E| = {mp.nstr(worst_eq5, 3)}; max |L recomputed - L| = {mp.nstr(worst_L, 3)}")
    print(f"rows matching a Belgrade gallery satellite (rule 1): {n_bg} of {len(rows)}; not matched: {len(not_bg)}")
    print(f"  not matched: {' '.join(not_bg)}")
    for tol, n in n_at.items():
        print(f"rows with an atlas candidate (rule 2) at tol {tol:g}: {n}")
    print(f"rows with an atlas row within 1e-4 in |L| |E|^(1/2) alone (period ignored, in case the atlas's T were an "
          f"absolute period): {n_lonly}")


if __name__ == "__main__":
    main()
