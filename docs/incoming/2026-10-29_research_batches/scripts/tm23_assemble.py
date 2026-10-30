"""tm23_assemble.py <dir>: assemble Tanikawa & Mikkola 2023 supplement Tables 2-8 from per-block readings.

<dir>/eye/<block>.txt holds the by-eye reading of one printed block (one row per line, cells
space-separated, in page order). <dir>/ocr_raw/<block>.txt and <block>_pad.txt hold two Vision OCR
passes over the same block image (ocrrows.py): as rendered at 300 dpi, and with an 80-pixel white
border and 150 % upscaling (Vision drops different rows in each). For p2R a third pass, p2R_mid.txt,
covers the middle third of the block (rows alpha2 = -0.6 to -5.0) cropped alone with a 60-pixel
border at 120 %, because both whole-block passes lost rows there. Blocks are the page halves of
supplement pp. 2-9 (p2L = page 2, left block).

The OCR reading is built from the two passes alone, without reference to the eye reading: a row is
kept when its first cell is a number and it has the full cell count (4, or 6 in Table 8), after
splitting cells on spaces; rows are matched across passes on their x cell (with an occurrence count);
where the passes that have the row agree (comparing with O, o and U read as 0) the first pass's text
is taken, and where they disagree the distinct readings are written joined by '|' so the diff shows
the conflict.

Writes supp_eye.csv and supp_ocr.csv: table, block, occ, x, y, t, s, alpha2, beta1 (t and s only in
Table 8). occ counts earlier rows of the block with the same x; the readings pair on block, x, occ.
"""
import csv, os, re, sys
d = sys.argv[1]
BLOCKS = [("2", "p2L"), ("2", "p2R"), ("3", "p3L"), ("3", "p3R"), ("4", "p4L"), ("4", "p4R"),
          ("4", "p5L"), ("4", "p5R"), ("5", "p6L"), ("5", "p6R"), ("6", "p7L"), ("6", "p7R"),
          ("7", "p8L"), ("8", "p9L"), ("8", "p9R")]
NUM = re.compile(r"^[-−]?\d")
HDR = ["table", "block", "occ", "x", "y", "t", "s", "alpha2", "beta1"]

def rows(path, ocr, n):
    out, seen = [], {}
    for line in open(path):
        cells = [w for c in line.rstrip("\n").split("\t" if ocr else None) for w in c.split()]
        if cells and NUM.match(cells[0]) and (not ocr or len(cells) == n):
            k = seen[cells[0]] = seen.get(cells[0], 0) + 1
            out.append(((cells[0], k), cells))
    return out

def write(name, table):
    with open(os.path.join(d, name), "w", newline="") as f:
        w = csv.writer(f, lineterminator="\n"); w.writerow(HDR)
        for tab, blk, (x, occ), c in table:
            if tab == "8":
                x, y, t, s, a, b = c
            else:
                x, y, a, b = c; t = s = ""
            w.writerow([tab, blk, occ, x, y, t, s, a, b])

eye, ocr = [], []
for tab, blk in BLOCKS:
    n = 6 if tab == "8" else 4
    for key, c in rows(os.path.join(d, "eye", blk + ".txt"), False, n):
        eye.append((tab, blk, key, c))
    passes = [dict(rows(os.path.join(d, "ocr_raw", blk + suf + ".txt"), True, n))
              for suf in ("", "_pad", "_mid") if os.path.exists(os.path.join(d, "ocr_raw", blk + suf + ".txt"))]
    order = []
    for p in passes:
        order += [k for k in p if k not in order]
    z = lambda v: re.sub(r"[OoU]", "0", v) if re.fullmatch(r"[-0-9OoU.]+", v) else v
    for key in order:
        cands = [p[key] for p in passes if key in p]
        c = []
        for cells in zip(*cands):
            uniq = []
            for v in cells:
                if z(v) not in [z(u) for u in uniq]:
                    uniq.append(v)
            c.append("|".join(uniq))
        ocr.append((tab, blk, key, c))
write("supp_eye.csv", eye)
write("supp_ocr.csv", ocr)
