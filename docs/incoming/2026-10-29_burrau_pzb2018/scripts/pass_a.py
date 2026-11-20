# Pass A: parse Table 2 from `pdftotext -layout` output (page 9 of arXiv:1802.00970v1).
import re, sys, csv
src, out = sys.argv[1], sys.argv[2]
lines = open(src, encoding="utf-8").read().splitlines()
start = next(i for i, l in enumerate(lines) if l.strip().startswith("Table 2."))
num = r"-?\d+\.\d+(?:e-?\d+)?"
run = t = None
pending = None  # (mass, x, vx)
rows = []
for l in lines[start:]:
    s = l.strip()
    if s.startswith("Unperturbed configurations"): run = "unperturbed"; continue
    if s.startswith("Perturbed configurations"): run = "perturbed"; continue
    if s.startswith("Midpoint (t=100)"): t = "100"; continue
    if s.startswith("End (t=200)"): t = "200"; continue
    m = re.fullmatch(rf"([345])\s+({num})\s+({num})", s)
    if m:
        assert pending is None, l
        pending = (m.group(1), m.group(2), m.group(3)); continue
    m = re.fullmatch(rf"({num})\s+({num})", s)
    if m:
        assert pending is not None, l
        rows.append([run, t, pending[0], pending[1], m.group(1), pending[2], m.group(2)])
        pending = None
        if len(rows) == 12: break
assert len(rows) == 12, len(rows)
with open(out, "w", newline="") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["run", "t", "mass", "x", "y", "vx", "vy"])
    w.writerows(rows)
