"""ocr2csv.py <raw_rows.txt> <header,comma,separated> <out.csv> [--keep REGEX] [--right] [--group REGEX] [--split]

Turns ocrrows.py output (tab-separated rows, the OCR of a page image) into a CSV reading. Rows whose
first cell matches --keep (default: any row with as many cells as the header) are kept; short rows
are padded with empty cells (on the right, or with --right on the left, for tables whose short
leading cells such as a lone "1" the OCR tends to miss) so the diff shows the gap; other rows are dropped and listed on stderr.
With --split, cells are also split on spaces (the OCR sometimes returns two adjacent numbers as one
box). With --group, a row whose text matches REGEX (a sub-heading such as "Body 1 is slingshot") is not
kept but sets the value of the first header column for the rows after it.
No value is edited: the cells are the OCR text as recognised.
"""
import csv, re, sys
raw, header, out = sys.argv[1], sys.argv[2].split(","), sys.argv[3]
keep = re.compile(sys.argv[sys.argv.index("--keep") + 1]) if "--keep" in sys.argv else None
with open(out, "w", newline="") as f:
    w = csv.writer(f, lineterminator="\n"); w.writerow(header)
    group = re.compile(sys.argv[sys.argv.index("--group") + 1]) if "--group" in sys.argv else None
    g = ""
    for line in open(raw):
        cells = line.rstrip("\n").split("\t")
        if "--split" in sys.argv:
            cells = [w for c in cells for w in c.split()]
        if group and group.search(" ".join(cells)):
            g = " ".join(cells); continue
        if group:
            cells = [g] + cells
        ok = keep.search(cells[1 if group else 0]) if keep else len(cells) == len(header)
        if ok and len(cells) <= len(header):
            pad = [""] * (len(header) - len(cells))
            w.writerow(pad + cells if "--right" in sys.argv else cells + pad)
        else:
            print("dropped: %r" % cells, file=sys.stderr)
