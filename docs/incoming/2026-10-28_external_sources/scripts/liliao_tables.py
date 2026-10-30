"""Readings A and B of Li & Liao (2018, arXiv:1805.07980v1) Tables A1-A5, and the overlap with OC26.

Usage:
  python3 -I liliao_tables.py read-a <pdftotext -layout output> <out.csv>
  python3 -I liliao_tables.py read-b <out.csv> <ocr.tsv>...   (ocr.swift output, one file per page, in page order)
  python3 -I liliao_tables.py read-b-pooled <out.csv> <pooled.tsv>...   (pool_strips.py output of 600-dpi strips of the
      table column: strip 0 = label, x and y, strip 1 = T; one file per page)
  python3 -I liliao_tables.py overlap <reconciled.csv> <OC26 threebodyorbits_ics.csv> <out.csv>

Reading A is the PDF's text layer; reading B is Vision OCR of the rendered pages (300 dpi). For B, OCR boxes are
split into the page's two columns (box left edge < 0.5 of the width), grouped into lines by mid-y within each
column, and read column by column, top to bottom. The OCR garbles the row labels' digits (F2 as Fz, F5 as Fs, ...),
so B numbers its rows F1, F2, ... in printed order within each mass set; the labels of reading A are checked to be
that same sequence. The OCR's numbers, masses and order are used unaltered.
Each row: Fi (m1, m2, m3) x y T, with r1 = (-0.5, 0), r2 = (0.5, 0), r3 = (x, y), all at rest, G = 1 (captions).
Values are kept as printed strings (the paper drops trailing zeros, so some cells have 9 decimals).

Overlap rule: an OC26 row of family `freefall` matches a paper row when its masses (m1, m2, m3) are equal as
numbers, its code equals the paper's Fi, and its published (x3, y3) equal the paper's (x, y) as decimals; the
published (x1, y1, x2, y2) must be (-0.5, 0, 0.5, 0) and all published velocities 0. A paper row whose (masses,
x, y) appear in OC26 under another code is reported as "present under another code". OC26's T is compared
with the paper's T and the difference reported, not used for matching (the atlas's T is its own refinement).
"""
import csv
import re
import sys
from decimal import Decimal

ROW = re.compile(r"F(\d+)\s*\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*\)\s+(-?\d+\.\d+)\s+(-?\d+\.\d+)\s+(\d+\.\d+)")
FIELDS = ["table", "Fi", "m1", "m2", "m3", "x", "y", "T"]


def table_of(m1, m2, m3):
    # the table each mass set is printed in (checked on the page images)
    return {("1", "1", "1"): "A1", ("1", "0.8", "0.8"): "A1", ("1", "0.8", "0.6"): "A2",
            ("1", "0.8", "0.4"): "A3", ("1", "0.8", "0.2"): "A4", ("0.6", "0.8", "1"): "A5"}.get((m1, m2, m3), "?")


def rows_from_text(lines):
    out = []
    for l in lines:
        l = l.replace("−", "-")
        for m in ROW.finditer(l):
            fi, m1, m2, m3, x, y, T = m.groups()
            out.append(dict(Fi=f"F{fi}", m1=m1, m2=m2, m3=m3, x=x, y=y, T=T))
    return out


def write(rows, path):
    for r in rows:
        r["table"] = table_of(r["m1"], r["m2"], r["m3"])
    with open(path, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=FIELDS, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


def key(r):
    return f"{r['m1']}|{r['m2']}|{r['m3']}|{r['Fi']}"


def read_a(src, out):
    text = open(src, encoding="utf-8").read().splitlines()
    start = next(i for i, l in enumerate(text) if "TABLE A1." in l)
    end = next(i for i, l in enumerate(text) if "TABLE A6." in l)
    rows = rows_from_text(text[start:end])
    write(rows, out)
    print(len(rows), "rows (reading A)")


ROWB = re.compile(r"F\S{0,3}\s*\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*\)\s*(-?\d+\.\d+)\s+(-?\d+\.\d+)\s+(\d+\.\d+)")


def read_b(out, pages, rowtol=0.004, pooled=False):
    rows = []
    unparsed = []
    for p in pages:
        boxes = []
        for line in open(p, encoding="utf-8"):
            q = line.rstrip("\n").split("\t")
            if len(q) < 4:
                continue
            col = False if pooled else float(q[1]) >= 0.5
            boxes.append((col, -float(q[0]), float(q[1]), q[3].replace("\u2212", "-")))
        boxes.sort()
        lines, cur, last = [], [], None
        for col, ny, x0, t in boxes:
            if cur and (col != cur[-1][0] or ny - last > rowtol):
                lines.append(" ".join(b[3] for b in sorted(cur, key=lambda b: b[2])))
                cur = []
            cur.append((col, ny, x0, t))
            last = ny
        if cur:
            lines.append(" ".join(b[3] for b in sorted(cur, key=lambda b: b[2])))
        for l in lines:
            m = ROWB.search(l)
            if m:
                m1, m2, m3, x, y, T = m.groups()
                rows.append(dict(m1=m1, m2=m2, m3=m3, x=x, y=y, T=T))
            elif re.match(r"^F\S{0,3}\s*\(", l):
                unparsed.append(l)
    seen = {}
    for r in rows:
        k = (r["m1"], r["m2"], r["m3"])
        seen[k] = seen.get(k, 0) + 1
        r["Fi"] = f"F{seen[k]}"
    write(rows, out)
    print(len(rows), "rows (reading B);", len(unparsed), "row-like lines not parsed:")
    for l in unparsed:
        print("  UNPARSED", l)


def overlap(rec, ics, out):
    paper = list(csv.DictReader(open(rec)))
    cat = [r for r in csv.DictReader(open(ics)) if r["family"] == "freefall"]
    D = Decimal
    res = []
    counts = {}
    for p in paper:
        pm = (D(p["m1"]), D(p["m2"]), D(p["m3"]))
        cands = [c for c in cat if (D(c["m1"]), D(c["m2"]), D(c["m3"])) == pm]
        same_xy = [c for c in cands if D(c["pub_x3"]) == D(p["x"]) and D(c["pub_y3"]) == D(p["y"])]
        same_code = [c for c in same_xy if c["code"] == p["Fi"]]
        if same_code:
            c = same_code[0]
            ok_frame = (D(c["pub_x1"]), D(c["pub_y1"]), D(c["pub_x2"]), D(c["pub_y2"])) == (D("-0.5"), 0, D("0.5"), 0) and \
                all(D(c[k]) == 0 for k in ("pub_vx1", "pub_vy1", "pub_vx2", "pub_vy2", "pub_vx3", "pub_vy3"))
            status = "present" if ok_frame else "present, frame differs"
            dT = f"{float(D(c['T']) - D(p['T'])):.3e}"
            ck = c["key"]
        elif same_xy:
            status, dT, ck = "present under another code", "", same_xy[0]["key"]
        else:
            status, dT, ck = "not in OC26", "", ""
        counts[status] = counts.get(status, 0) + 1
        res.append(dict(table=p["table"], Fi=p["Fi"], m1=p["m1"], m2=p["m2"], m3=p["m3"], status=status,
                        oc26_key=ck, T_oc26_minus_T_paper=dT))
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(res[0].keys()), lineterminator="\n")
        w.writeheader()
        w.writerows(res)
    print(len(paper), "paper rows;", counts)


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "read-a":
        read_a(sys.argv[2], sys.argv[3])
    elif cmd == "read-b-pooled":
        read_b(sys.argv[2], sys.argv[3:], pooled=True)
    elif cmd == "read-b":
        read_b(sys.argv[2], sys.argv[3:])
    else:
        overlap(sys.argv[2], sys.argv[3], sys.argv[4])
