"""Reading B of Li & Liao (2018) Tables A1-A5 by consensus of three OCR passes of the page images.

Usage: python3 -I liliao_ocr_consensus.py <out.csv> <eye_needed.csv> <page>:<full.tsv>,<column.tsv>,<pooled.tsv> ...

Passes (all ocr.swift output; mid-y is a fraction of the full page height in all three):
  full   = the whole page at 300 dpi (boxes with left edge < 0.5, the table column);
  column = the table column (page x 0.07-0.51) cropped from a 600-dpi render;
  pooled = two strips of the 600-dpi render (label+x+y, and T) upscaled 2x, pooled by pool_strips.py.
Each pass is grouped into lines by mid-y (tolerance 0.004); a line is a table row if it holds three decimals
(x, y with 6+ decimals and T with 3+). Lines of the three passes are aligned by mid-y (tolerance 0.004).
A cell is taken when at least two passes give the identical string; otherwise it is written as EYE and listed
in eye_needed.csv, to be read by eye from the page image. The mass set is read from the label "(m1, m2, m3)"
by the same two-of-three rule, else carried from the previous row (the tables list one mass set at a time).
Fi is numbered in printed order within each mass set (the OCR garbles the labels' subscripts).
Nothing is compared with reading A here.
"""
import csv
import re
import sys

NUMS = re.compile(r"(\d\.\d{6,})\s+(\d\.\d{6,})\s+(\d+\.\d{3,})")
MASS = re.compile(r"\(\s*(1|0\.\d)\s*,\s*(1|0\.\d)\s*,\s*(1|0\.\d)\s*\)")


def lines_of(path, left_only, tol=0.004):
    boxes = []
    for l in open(path, encoding="utf-8"):
        q = l.rstrip("\n").split("\t")
        if len(q) < 4:
            continue
        if left_only and float(q[1]) >= 0.5:
            continue
        boxes.append((-float(q[0]), float(q[1]), q[3].replace("−", "-")))
    boxes.sort()
    out, cur, last = [], [], None
    for ny, x0, t in boxes:
        if cur and ny - last > tol:
            out.append(cur)
            cur = []
        cur.append((ny, x0, t))
        last = ny
    if cur:
        out.append(cur)
    res = []
    for c in out:
        text = " ".join(t for _, _, t in sorted(c, key=lambda b: b[1]))
        y = -sum(b[0] for b in c) / len(c)
        m = NUMS.search(text)
        mm = MASS.search(text)
        res.append(dict(y=y, nums=m.groups() if m else None, mass=mm.groups() if mm else None, text=text))
    return [r for r in res if r["nums"] or r["mass"]]


def vote(vals):
    vals = [v for v in vals if v is not None]
    for v in vals:
        if vals.count(v) >= 2:
            return v
    return None


def main(out, eyeout, specs):
    rows, eye = [], []
    prev_mass = None
    for spec in specs:
        page, files = spec.split(":", 1)
        full, col, pooled = files.split(",")
        passes = [lines_of(full, True), lines_of(col, False), lines_of(pooled, False)]
        # anchor on the union of row y positions
        ys = sorted({round(r["y"], 4) for p in passes for r in p if r["nums"]}, reverse=True)
        anchors = []
        for y in ys:
            if not anchors or anchors[-1] - y > 0.004:
                anchors.append(y)
        for y in anchors:
            got, gotm = [], []
            for p in passes:
                hit = [r for r in p if abs(r["y"] - y) <= 0.004 and r["nums"]]
                got.append(min(hit, key=lambda r: abs(r["y"] - y)) if hit else None)
                hm = [r for r in p if abs(r["y"] - y) <= 0.004 and r["mass"]]
                gotm.append(min(hm, key=lambda r: abs(r["y"] - y))["mass"] if hm else None)
            mass = vote(gotm) or prev_mass
            prev_mass = mass
            cells = {}
            for i, name in enumerate(("x", "y", "T")):
                v = vote([g["nums"][i] if g and g["nums"] else None for g in got])
                if v is None:
                    v = "EYE"
                    eye.append(dict(page=page, ypos=f"{y:.4f}", field=name,
                                    readings=" | ".join(g["nums"][i] if g and g["nums"] else "-" for g in got)))
                cells[name] = v
            rows.append(dict(page=page, ypos=f"{y:.4f}", m1=mass[0], m2=mass[1], m3=mass[2], **cells))
    seen = {}
    for r in rows:
        k = (r["m1"], r["m2"], r["m3"])
        seen[k] = seen.get(k, 0) + 1
        r["Fi"] = f"F{seen[k]}"
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["page", "ypos", "Fi", "m1", "m2", "m3", "x", "y", "T"],
                           lineterminator="\n")
        w.writeheader()
        w.writerows(rows)
    with open(eyeout, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["page", "ypos", "field", "readings"], lineterminator="\n")
        w.writeheader()
        w.writerows(eye)
    counts = {}
    for r in rows:
        counts[(r["m1"], r["m2"], r["m3"])] = counts.get((r["m1"], r["m2"], r["m3"]), 0) + 1
    print(len(rows), "rows;", len(eye), "cells need eye;", counts)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3:])
