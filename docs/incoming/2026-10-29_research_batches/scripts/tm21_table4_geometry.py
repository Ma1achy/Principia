"""tm21_table4_geometry.py <tanikawa_mikkola_2021 dir>: check Tanikawa & Mikkola (2021) Table 4's geometry
(paper arithmetic only, no integration).

Table 4's nine non-collision controls are TSM point 1 (Table 2 row No. 1, the point (x0, y0)) displaced by the
printed radius r at the printed azimuth az. For each row this prints the radius label and azimuth as the page
gives them, the computed point (x0 + r cos az, y0 + r sin az) to 8 decimals, and the residual printed - computed.

Run: python3 -I scripts/tm21_table4_geometry.py tanikawa_mikkola_2021 > tanikawa_mikkola_2021/table4_geometry_check.txt
"""
import csv, math, os, sys
d = sys.argv[1]
rd = lambda n: list(csv.DictReader(open(os.path.join(d, n), encoding="utf-8")))
p1 = [r for r in rd("table2_reconciled.csv") if r["no"] == "1"][0]
x0, y0 = float(p1["x"]), float(p1["y"])
for r in rd("table4_reconciled.csv"):
    rad = float(r["radius"].split(":")[1])
    az = math.radians(float(r["azimuth_deg"]))
    xc, yc = x0 + rad * math.cos(az), y0 + rad * math.sin(az)
    print("%s %s %.8f %.8f dx=%.1e dy=%.1e" % (r["radius"], r["azimuth_deg"], xc, yc,
                                               float(r["x"]) - xc, float(r["y"]) - yc))
