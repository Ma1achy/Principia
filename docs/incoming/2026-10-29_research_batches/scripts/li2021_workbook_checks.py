"""li2021_workbook_checks.py <li2021_xlsx_ics.csv> <threebodyorbits_ics.csv>

Checks on the parsed Li, Li, He & Liao 2021 workbook (no integration, no engine code):
 1. every (x, y) lies in Agekyan & Anosova's region D as the paper's eq. (1) defines it;
 2. ranges of x, y, T and of the symbol-sequence length; duplicate (x, y) pairs;
 3. frame check: the 11 triple-collision points of Tanikawa, Saito & Mikkola 2019, Table 1 (manuscript
    p. 24, read from the page image for this check only; task 15 owns their transcription), matched to
    the nearest workbook row. Li 2021 §3.2 says the 1653 include these 11, so in the same frame each
    should have a workbook row within the 8-digit rounding of TSM's table (5e-9) plus a margin;
 4. exclusion overlap: against the threebodyorbits snapshot (docs/incoming/2026-10-26_orbit_catalogues/),
    every row with three equal masses and all six velocity components zero (the free-fall rows) is a
    candidate; its free body's (x, y), with the other two at (-0.5, 0) and (0.5, 0), is compared with
    each workbook (x, y) and its mirror (-x, y). Match rule: Euclidean distance < 1e-6.
"""
import csv, sys, math

wb = list(csv.DictReader(open(sys.argv[1])))
P = [(float(r["x"]), float(r["y"]), float(r["T"])) for r in wb]
out = []
inD = [x >= 0 and y >= 0 and (x + 0.5) ** 2 + y ** 2 <= 1 for x, y, _ in P]
out.append("1. rows %d; in region D (x>=0, y>=0, (x+0.5)^2+y^2<=1): %d; outside: %s" % (
    len(P), sum(inD), [wb[i]["label"] for i, ok in enumerate(inD) if not ok]))
L = [len(r["symbol_sequence"]) for r in wb]
out.append("2. x in [%.6g, %.6g], y in [%.6g, %.6g], T in [%.6g, %.6g]; sequence length %d..%d; "
           "alphabet %s; duplicate (x,y): %d" % (
    min(p[0] for p in P), max(p[0] for p in P), min(p[1] for p in P), max(p[1] for p in P),
    min(p[2] for p in P), max(p[2] for p in P), min(L), max(L),
    sorted(set("".join(r["symbol_sequence"] for r in wb))), len(P) - len(set((p[0], p[1]) for p in P))))
# TSM 2019 Table 1 (No, x, y, t, digits), accepted manuscript p. 24, read by eye from the page image.
TSM = [(1, 0.19208270, 0.30936018, 1.35458, 8), (2, 0.22202750, 0.30096440, 1.30831, 8),
       (3, 0.10677930, 0.52012268, 1.83657, 8), (4, 0.08875296, 0.45639865, 1.99380, 10),
       (5, 0.15567083, 0.33309483, 1.67790, 10), (6, 0.15882908, 0.64735217, 2.90464, 11),
       (7, 0.08212247, 0.41682453, 2.16131, 12), (8, 0.27949737, 0.57593177, 3.04707, 13),
       (9, 0.19095011, 0.58286178, 3.08930, 13), (10, 0.09012264, 0.38213664, 2.30804, 14),
       (11, 0.10106723, 0.37459122, 2.28158, 14)]
out.append("3. TSM 2019 Table 1 against the workbook (nearest row; same frame, no mirror):")
for n, x, y, t, d in TSM:
    i = min(range(len(P)), key=lambda j: (P[j][0] - x) ** 2 + (P[j][1] - y) ** 2)
    dist = math.hypot(P[i][0] - x, P[i][1] - y)
    out.append("   TSM %2d (%.8f, %.8f) t=%.5f digits=%2d -> %-8s (%.10f, %.10f) T=%.6f len=%3d  dist %.1e  |dT| %.1e" % (
        n, x, y, t, d, wb[i]["label"], P[i][0], P[i][1], P[i][2], L[i], dist, abs(P[i][2] - t)))
cat = [r for r in csv.DictReader(open(sys.argv[2]))
       if r["m1"] == r["m2"] == r["m3"] == "1.0"
       and all(float(r["pub_v%s%d" % (c, k)] or 0) == 0 for c in "xy" for k in (1, 2, 3))]
fam = {}
hits = []
for r in cat:
    fam[r["family"]] = fam.get(r["family"], 0) + 1
    pts = [(float(r["pub_x%d" % k]), float(r["pub_y%d" % k])) for k in (1, 2, 3)]
    free = [p for p in pts if not (abs(abs(p[0]) - 0.5) < 1e-12 and p[1] == 0)]
    if len(free) != 1: continue
    fx, fy = free[0]
    for i, (x, y, _) in enumerate(P):
        for sx in (x, -x):
            if math.hypot(sx - fx, y - fy) < 1e-6:
                hits.append((r["key"], wb[i]["label"]))
out.append("4. threebodyorbits rows with equal unit masses at rest: %d %r; Li 2021 workbook rows matched "
           "(distance < 1e-6, either mirror): %d %r" % (len(cat), fam, len(hits), hits))
near = min((math.hypot(sx - float(f["pub_x3"]), y - float(f["pub_y3"])), f["key"], wb[i]["label"])
           for f in cat for i, (x, y, _) in enumerate(P) for sx in (x, -x))
out.append("   closest pair overall: distance %.2e (%s, %s)" % near)
print("\n".join(out))
