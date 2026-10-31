"""Reading B parser: turns ocr.swift output for one cropped table into a CSV grid.

Usage: python3 -I ocr_grid.py <ocr.tsv> <out.csv> <col1,col2,...> [--rowtol 0.006] [--minus] [--raw]
With --raw, every grouped row is written as one line of space-separated tokens (no column check); a table-specific
parser then reads those lines.
Text boxes are split on whitespace into tokens (each token given its box's mid-y). Tokens are grouped into rows by
mid-y (a new row starts when the gap to the previous token's mid-y exceeds rowtol), each row sorted left to right.
Rows whose token count differs from the column count are written with a leading '#BAD' comment line to stderr and
skipped; they are then read by eye. --minus maps the typographic minus (U+2212) and similar dashes to '-', and the
OCR's 'E-' spacing variants ('E -10', 'E–10') to 'E-', and the Cyrillic look-alikes the OCR emits (Е, З, а) to E, 3, a. Nothing else is altered: no digit is ever corrected here.
"""
import csv
import re
import sys


def main():
    src, out, cols = sys.argv[1], sys.argv[2], sys.argv[3].split(",")
    tol = float(sys.argv[sys.argv.index("--rowtol") + 1]) if "--rowtol" in sys.argv else 0.006
    minus = "--minus" in sys.argv
    toks = []
    with open(src, encoding="utf-8") as f:
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) < 4:
                continue
            y, x0, x1, text = float(p[0]), float(p[1]), float(p[2]), p[3]
            if minus:
                text = re.sub("[−–—]", "-", text)
                text = re.sub(r"E\s*-\s*(\d)", r"E-\1", text)
                text = text.replace("\u0415", "E").replace("\u0417", "3").replace("\u0430", "a")
            parts = text.split()
            for i, t in enumerate(parts):
                toks.append((y, x0 + (x1 - x0) * i / max(1, len(parts)), t))
    toks.sort(key=lambda t: -t[0])
    rows, cur, last = [], [], None
    for t in toks:
        if last is not None and last - t[0] > tol:
            rows.append(cur)
            cur = []
        cur.append(t)
        last = t[0]
    if cur:
        rows.append(cur)
    good = bad = 0
    if "--raw" in sys.argv:
        with open(out, "w", encoding="utf-8") as f:
            for r in rows:
                r.sort(key=lambda t: t[1])
                f.write(" ".join(t[2] for t in r) + "\n")
        print(f"{out}: {len(rows)} raw rows written", file=sys.stderr)
        return
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(cols)
        for r in rows:
            r.sort(key=lambda t: t[1])
            vals = [t[2] for t in r]
            if len(vals) != len(cols):
                print("#BAD\t" + " ".join(vals), file=sys.stderr)
                bad += 1
                continue
            w.writerow(vals)
            good += 1
    print(f"{out}: {good} rows written, {bad} rows skipped (wrong token count)", file=sys.stderr)


if __name__ == "__main__":
    main()
