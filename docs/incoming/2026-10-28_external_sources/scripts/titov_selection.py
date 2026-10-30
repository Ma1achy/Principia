"""Titov (2024) Appendix B: which orbits TI-1 keeps, by one rule, and the bound 7-decimal IC rounding puts on E and |J|.

Usage: python3 -I titov_selection.py <appB_reconciled.csv> <appB_EJ_recomputed.csv>

The rule. The appendix carries the ICs, so an orbit is kept only if
  (1) the appendix's own E and |J| check against the recomputation from its ICs (within one last-place unit, the
      criterion of titov_recompute_EJ.py), and
  (2) where a main-text table (2, 3 or 4) prints E or |J| to more decimals than the appendix, that more precise value
      checks too.
A main-text value printed to the same number of decimals as the appendix that disagrees while the appendix checks is
an error of the table's (listed), not of the orbit.

The bound. Every IC (x1, x2, x3, vy1, vy2, vy3) is printed to 7 decimals, so each carries a rounding of at most
0.5e-7. To first order E and J move by at most 0.5e-7 * sum |dQ/dq| over the six ICs, with (G = 1, y = xdot = 0)
  dE/dvy_i = m_i vy_i,  dE/dx_i = sum_{j != i} m_i m_j (x_i - x_j)/|x_i - x_j|^3,
  dJ/dvy_i = m_i x_i,   dJ/dx_i = m_i vy_i.
A disagreement larger than this bound is not explained by the ICs' rounding. Bounds are printed in units of 1e-6
(E) and 1e-5 (|J|), the main tables' last places.
"""
import csv
import sys

import mpmath as mp

mp.mp.dps = 50
HALF = mp.mpf("0.5e-7")


def dec_places(s):
    return len(s.split(".")[1]) if "." in s else 0


def bounds(r):
    m = [mp.mpf(r[k]) for k in ("m1", "m2", "m3")]
    x = [mp.mpf(r[k]) for k in ("x1", "x2", "x3")]
    v = [mp.mpf(r[k]) for k in ("vy1", "vy2", "vy3")]
    dE = [m[i] * v[i] for i in range(3)]
    dE += [sum(m[i] * m[j] * (x[i] - x[j]) / abs(x[i] - x[j]) ** 3 for j in range(3) if j != i) for i in range(3)]
    dJ = [m[i] * x[i] for i in range(3)] + [m[i] * v[i] for i in range(3)]
    return HALF * sum(abs(d) for d in dE), HALF * sum(abs(d) for d in dJ)


def main(src, rec):
    ics = {r["id"]: r for r in csv.DictReader(open(src, encoding="utf-8"))}
    kept, out, table_errors = [], [], []
    worst = (0, 0)
    for r in csv.DictReader(open(rec, encoding="utf-8")):
        bE, bJ = bounds(ics[r["id"]])
        reasons = []
        if r["E_checks"] != "yes" or r["J_checks"] != "yes":
            reasons.append("appendix E or |J| does not check")
        for q, ap, tv, tu, tc in (("E", r["E_printed"], r["table_E"], r["table_E_last_place_units"],
                                    r["table_E_checks"]),
                                   ("|J|", r["J_printed"], r["table_absJ"], r["table_J_last_place_units"],
                                    r["table_J_checks"])):
            if not tv or tc == "yes":
                continue
            if dec_places(tv) > dec_places(ap):
                reasons.append(f"table {q} {tv} ({dec_places(tv)} decimals, appendix {dec_places(ap)}) is "
                               f"{tu} units off")
            elif not reasons:
                table_errors.append(f"{r['id']} ({r['table_row']}): table {q} {tv} against appendix {ap}, "
                                    f"{tu} units; the appendix checks")
        print(f"{r['id']}: rounding bound E {mp.nstr(bE * 10**6, 3)}e-6, |J| {mp.nstr(bJ * 10**5, 3)}e-5"
              + (f"; EXCLUDED: {'; '.join(reasons)}" if reasons else "; kept"))
        if reasons:
            out.append(r["id"])
        else:
            kept.append(r["id"])
            worst = (max(worst[0], bE), max(worst[1], bJ))
    print(f"kept {len(kept)}: {' '.join(kept)}")
    print(f"excluded {len(out)}: {' '.join(out)}")
    print(f"largest rounding bound among the kept: E {mp.nstr(worst[0] * 10**6, 3)}e-6, "
          f"|J| {mp.nstr(worst[1] * 10**5, 3)}e-5")
    print("main-text table errors at the appendix's own precision (the orbit kept):")
    for t in table_errors:
        print(f"  {t}")


if __name__ == "__main__":
    main(*sys.argv[1:])
