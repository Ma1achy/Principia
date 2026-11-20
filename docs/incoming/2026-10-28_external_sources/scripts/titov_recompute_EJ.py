"""Recompute E and J of Titov (2024) Appendix B from the printed initial conditions.

Usage: python3 -I titov_recompute_EJ.py <appB_reconciled.csv> <out.csv> [<table2.csv> <table3.csv> <table4.csv>]

G = 1 (Titov's equations carry no G; the figure-eight's E = -1.29297 with
m = 1 is the G = 1 value). Every IC has y_i = 0 and xdot_i = 0, so
  E = sum m_i vy_i^2 / 2 - sum_{i<j} m_i m_j / |x_i - x_j|,
  J = sum m_i x_i vy_i,
evaluated in mpmath at 50 digits. Also reported: the centre of mass
sum m_i x_i and the momentum sum m_i vy_i, which should be ~0 to the printed
7 decimals. A tabulated value "checks" if it equals the recomputed one rounded
to the printed number of decimals, or differs by at most 1 unit in its last
printed place (the ICs themselves carry 7-decimal rounding: the column
last_place_units gives the difference in units of the last printed place).
The appendix prints J; Tables 2-4 print |C| or |J|. The comparison is made on
|J| and the sign of the recomputed J is reported in J_sign.
With the main-body tables given, each appendix orbit is matched to the table
row with the same action A (string equal, else the nearest A within 2e-5, the
match then flagged) and that row's E and |J| are compared too.
"""
import csv
import sys

import mpmath as mp

mp.mp.dps = 50


def dec_places(s):
    return len(s.split(".")[1]) if "." in s else 0


def main(src, out, t2=None, t3=None, t4=None):
    rows = list(csv.DictReader(open(src)))
    tables = None
    if t2:
        tables = {"B2": list(csv.DictReader(open(t2))), "B3": list(csv.DictReader(open(t3))),
                  "T4": list(csv.DictReader(open(t4)))}
    res = []
    for r in rows:
        m = [mp.mpf(r[k]) for k in ("m1", "m2", "m3")]
        x = [mp.mpf(r[k]) for k in ("x1", "x2", "x3")]
        v = [mp.mpf(r[k]) for k in ("vy1", "vy2", "vy3")]
        T = sum(m[i] * v[i] ** 2 for i in range(3)) / 2
        U = -sum(m[i] * m[j] / abs(x[i] - x[j]) for i in range(3) for j in range(i + 1, 3))
        E = T + U
        Jsigned = sum(m[i] * x[i] * v[i] for i in range(3))
        J = abs(Jsigned)
        com = sum(m[i] * x[i] for i in range(3))
        mom = sum(m[i] * v[i] for i in range(3))
        out_row = dict(id=r["id"], E_printed=r["E"], E_recomputed=mp.nstr(E, 12),
                       J_printed=r["J"], J_recomputed=mp.nstr(J, 12),
                       J_sign="+" if Jsigned > 0 else "-",
                       com_x=mp.nstr(com, 3), momentum_y=mp.nstr(mom, 3))
        for q, val in (("E", E), ("J", J)):
            p = dec_places(r[q])
            units = (val - abs(mp.mpf(r[q])) if q == "J" else val - mp.mpf(r[q])) * mp.mpf(10) ** p
            out_row[f"{q}_last_place_units"] = mp.nstr(units, 4)
            out_row[f"{q}_checks"] = "yes" if abs(units) <= 1 else "NO"
        if tables:
            t = tables[r["id"][:2]]
            hit = [x for x in t if x["A"] == r["A"]]
            flag = ""
            if not hit:
                hit = [x for x in t if abs(mp.mpf(x["A"]) - mp.mpf(r["A"])) <= mp.mpf("2e-5")]
                flag = "A differs"
            if hit:
                x = hit[0]
                out_row["table_row"] = f"{r['id'][:2]}:{x['row']}" + (f" ({flag}: {x['A']})" if flag else "")
                out_row["table_E"], out_row["table_absJ"] = x["E"], x["absJ"]
                for q, val, tv in (("E", E, x["E"]), ("J", J, x["absJ"])):
                    p = dec_places(tv)
                    units = (val - mp.mpf(tv)) * mp.mpf(10) ** p
                    out_row[f"table_{q}_last_place_units"] = mp.nstr(units, 4)
                    out_row[f"table_{q}_checks"] = "yes" if abs(units) <= 1 else "NO"
            else:
                out_row["table_row"] = "none"
        res.append(out_row)
    fields = ["id", "E_printed", "E_recomputed", "E_last_place_units", "E_checks",
              "J_printed", "J_recomputed", "J_sign", "J_last_place_units", "J_checks",
              "com_x", "momentum_y"]
    if tables:
        fields += ["table_row", "table_E", "table_E_last_place_units", "table_E_checks",
                   "table_absJ", "table_J_last_place_units", "table_J_checks"]
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, restval="")
        w.writeheader()
        w.writerows(res)
    bad = [o["id"] for o in res if o["E_checks"] != "yes" or o["J_checks"] != "yes"]
    print(f"{len(res)} rows; appendix E or |J| failing: {len(bad)}: {' '.join(bad)}")
    if tables:
        tb = [o["id"] for o in res if o.get("table_E_checks") != "yes" or o.get("table_J_checks") != "yes"]
        print(f"table E or |J| failing or unmatched: {len(tb)}: {' '.join(tb)}")


if __name__ == "__main__":
    main(*sys.argv[1:])
