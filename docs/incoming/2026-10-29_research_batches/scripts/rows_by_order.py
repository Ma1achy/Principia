"""rows_by_order.py <ocr_rows.txt> <eye.csv> <out.csv> <first_data_line> [--label]: lay an OCR reading out
in the eye reading's rows, by order. From line <first_data_line> (1-based) on, each OCR line gives the numeric
tokens of one row ('- 1.4' joined to '-1.4'); with --label the eye row's first cell (a label) is copied and the
OCR numbers fill the remaining columns. A short OCR row is padded on the LEFT when its first number does not
match the eye row's first number, else on the right, so a dropped cell shows as an empty-cell disagreement.
No token is edited.
"""
import csv, re, sys
raw, eye, out, first = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
lab = "--label" in sys.argv
E = list(csv.reader(open(eye)))
L = open(raw, encoding="utf-8").read().splitlines()[first - 1:]
w = csv.writer(open(out, "w", newline=""), lineterminator="\n"); w.writerow(E[0])
for er, line in zip(E[1:], L):
    cells = line.replace("- ", "-").split("\t")
    if lab and cells and not re.fullmatch(r"-?\.?\d[\d.]*(?:E-?\d+)?", cells[0].strip()):
        cells = cells[1:]                      # the OCR'd row label (e.g. 'a1') is not data
    toks = [t for c in cells for t in re.findall(r"-?\.?\d[\d.]*(?:E-?\d+)?", c)]
    n = len(er) - (1 if lab else 0)
    if len(toks) < n:
        pad = [""] * (n - len(toks))
        toks = toks + pad if toks and toks[0] == er[1 if lab else 0] else pad + toks
    w.writerow(([er[0]] if lab else []) + toks[:n])
