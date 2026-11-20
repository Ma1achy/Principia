"""ocr_join_pairs.py <raw_rows.txt> <header,comma,separated> <out.csv> <n_first> [--first REGEX]

For tables printed with each record on two lines (Tanikawa & Mikkola 2021, Table 2: the edge labels on
the first line, their angles and the side on the second). A line matching --first (default: starts
with an integer) opens a record and should carry n_first cells (if it has fewer, as when the OCR
missed a short leading cell such as the row number, it is padded on the left); the next line supplies the rest. Cells
are the OCR text unedited; a record with the wrong cell count is still written, padded, so the diff
shows it.
"""
import csv, re, sys
raw, header, out, n1 = sys.argv[1], sys.argv[2].split(","), sys.argv[3], int(sys.argv[4])
first = re.compile(sys.argv[sys.argv.index("--first") + 1] if "--first" in sys.argv else r"^\d+$")
lines = [l.rstrip("\n").split("\t") for l in open(raw)]
recs, cur = [], None
for c in lines:
    if first.search(c[0]):
        cur = [""] * max(0, n1 - len(c)) + list(c); recs.append(cur)
    elif cur is not None and len(cur) < len(header):
        cur.extend(c)
with open(out, "w", newline="") as f:
    w = csv.writer(f, lineterminator="\n"); w.writerow(header)
    for r in recs:
        w.writerow((r + [""] * len(header))[:len(header)])
