"""t2000_vs_li2021.py <tanikawa_2000/reconciled dir> <li2021_xlsx_ics.csv>: Tanikawa 2000's triple-collision points
(Tables I-IV) against Li et al. 2021's 1653 free-fall triple-collision ICs, same frame (body 1 at (x, y) in D, bodies
2 and 3 at (-0.5, 0), (0.5, 0), equal masses at rest). Nearest workbook row per point, its distance, and the ratio of
Tanikawa's 'time' to Li's T (m = 1, G = 1) where the points coincide: a constant ratio would fix Tanikawa's time unit.
Arithmetic on published numbers only.
"""
import csv, math, os, sys
d, li = sys.argv[1], list(csv.DictReader(open(sys.argv[2])))
pts = [(r["label"], float(r["x"]), float(r["y"]), float(r["T"])) for r in li]
for t in ("I", "II", "III", "IV"):
    for r in csv.DictReader(open(os.path.join(d, "table_%s.csv" % t))):
        x, y, tm = float(r["x"]), float(r["y"]), float(r["time"])
        lab, lx, ly, lT = min(pts, key=lambda p: (p[1] - x) ** 2 + (p[2] - y) ** 2)
        dist = math.hypot(lx - x, ly - y)
        print("Table %-3s n_min %-3s (%.7f, %.7f) time %.7f | %-8s dist %.1e  T_Li %.7f  time/T_Li %.6f%s" % (
            t, r["n_min"], x, y, tm, lab, dist, lT, tm / lT, "  <- same point" if dist < 1e-5 else ""))
