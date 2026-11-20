# Pass B: Table 2 from Apple Vision OCR of the rendered page image (600 dpi, rotated upright),
# two column crops, lines sorted top to bottom; structure (run, t, mass) from the printed table's fixed order.
import re, sys, csv
pos_f, vel_f, out = sys.argv[1], sys.argv[2], sys.argv[3]
num = re.compile(r"-?\d+\.\d+(?:e-\d+)?")
def vals(p):
    r = []
    for l in open(p, encoding="utf-8"):
        y, x, txt = l.rstrip("\n").split("\t", 2)
        if num.fullmatch(txt.strip()): r.append((float(y), txt.strip()))
    r.sort(key=lambda a: -a[0])
    return [v for _, v in r]
P, V = vals(pos_f), vals(vel_f)
assert len(P) == 24 and len(V) == 24, (len(P), len(V))
order = [("unperturbed","100"),("unperturbed","200"),("perturbed","100"),("perturbed","200")]
rows = []
k = 0
for run, t in order:
    for mass in ("3","4","5"):
        rows.append([run, t, mass, P[k], P[k+1], V[k], V[k+1]]); k += 2
with open(out, "w", newline="") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["run", "t", "mass", "x", "y", "vx", "vy"])
    w.writerows(rows)
