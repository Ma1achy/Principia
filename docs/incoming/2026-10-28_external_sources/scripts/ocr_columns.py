"""Reading B parser for tables OCR'd one column strip at a time (ocr.swift on each vertical strip of one crop).

Usage: python3 -I ocr_columns.py <out.csv> <rowtol> <col1>=<strip1.tsv> <col2>=<strip2.tsv> ...
All strips are cut from the same crop at full height, so their normalised mid-y values share one scale. The first
column's tokens are the row anchors (keep only those matching --anchor-re, default a whole number). Each other
strip's tokens are assigned to the nearest anchor within rowtol; a strip token with no anchor in range, or two tokens
on one anchor, is reported. Typographic minus signs and dashes become '-', and 'E -10' spacing becomes 'E-10'.
No digit is altered. Missing cells are left empty and reported: they are read by eye.
"""
import csv
import re
import sys


def read(path):
    out = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) < 4:
                continue
            t = re.sub("[−–—]", "-", p[3])
            t = re.sub(r"E\s*-\s*(\d)", r"E-\1", t)
            t = re.sub(r"^-\s+(\d)", r"-\1", t)
            out.append((float(p[0]), t.strip()))
    return out


def main():
    out, tol = sys.argv[1], float(sys.argv[2])
    anchor_re = re.compile(r"^\d+$")
    specs = [a.split("=", 1) for a in sys.argv[3:]]
    cols = [c for c, _ in specs]
    anchors = sorted([t for t in read(specs[0][1]) if anchor_re.match(t[1])], key=lambda t: -t[0])
    grid = [[a[1]] + [""] * (len(cols) - 1) for a in anchors]
    problems = 0
    for j, (c, path) in enumerate(specs[1:], start=1):
        for y, t in read(path):
            best = min(range(len(anchors)), key=lambda i: abs(anchors[i][0] - y)) if anchors else None
            if best is None or abs(anchors[best][0] - y) > tol:
                print(f"UNASSIGNED\t{c}\ty={y:.4f}\t{t}", file=sys.stderr)
                problems += 1
                continue
            if grid[best][j]:
                print(f"COLLISION\trow {grid[best][0]}\t{c}\t{grid[best][j]!r} and {t!r}", file=sys.stderr)
                problems += 1
                grid[best][j] += " " + t
            else:
                grid[best][j] = t
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(cols)
        for r in grid:
            for j, v in enumerate(r):
                if not v:
                    print(f"EMPTY\trow {r[0]}\t{cols[j]}", file=sys.stderr)
                    problems += 1
            w.writerow(r)
    print(f"{out}: {len(grid)} rows; {problems} problems reported", file=sys.stderr)


if __name__ == "__main__":
    main()
