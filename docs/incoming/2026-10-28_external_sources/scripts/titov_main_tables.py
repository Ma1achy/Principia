"""Readings A and B of Titov (2024) Tables 2, 3 and 4 (pp. 51 and 59).

Usage: python3 -I titov_main_tables.py <pdftotext -layout output> <p51.raw> <p59.raw> <outdir>

Reading A: rows of the PDF text layer. Reading B: rows of Vision OCR of the
300-dpi page images (ocr_grid.py --raw output). The OCR drops the narrow
stability marks ('+'/'-', column Ust) on some rows; those were read by eye
from the page crops and are supplied below (EYE_UST), all other B cells are
OCR tokens unchanged.
Column headers (translated): A = action functional; E = energy constant;
|C| (Table 2) or |J| (Tables 3, 4) = absolute angular momentum; omega =
angular velocity of the rotating frame; [Imin,Imax] = range of the moment of
inertia; Fig = figure number (л/с/п = left/centre/right panel); Ust =
'stability' mark: '+' if the orbit survives a long time (several thousand
periods), '-' otherwise (p. 59).
"""
import csv
import os
import re
import sys

ROW2 = re.compile(r"^\s*(\d+\.\d{5})\s+(-?\d\.\d+)\s+(\d\.\d+)\s+(\d/\d)\s+(\S+?)\s+(\d\.\d+[лсcп])\s*([+-])?\s*$")
ROW4 = re.compile(r"^\s*(0\.\d+)\s+(\d+\.\d{5})\s+(-?\d\.\d+)\s+(\d\.\d+)\s+(\[\S+\])\s+(\d\.\d+[лсcпn]?)\s*$")
# '-' marks the OCR missed (rows of Table 2 and Table 3, keyed by A), read by eye from the crops
EYE_UST = {"12.43822": "+", "16.57031": "-", "17.61955": "-", "16.61662": "-",
           "14.08066": "-", "14.09948": "-", "14.55725": "-", "16.64808": "-", "17.80747": "-"}


def norm(s):
    s = s.replace("−", "-").replace("с", "c")  # Cyrillic es -> latin c in figure refs
    return s


def parse(lines, table4=False):
    out = []
    for l in lines:
        l = norm(l)
        m = (ROW4 if table4 else ROW2).match(l)
        if m:
            out.append(list(m.groups()))
    return out


def main(txt, r51, r59, outdir):
    t = open(txt, encoding="utf-8").read().splitlines()
    i2 = next(i for i, l in enumerate(t) if l.startswith("Таблица 2 "))
    i3 = next(i for i, l in enumerate(t) if l.strip() == "Таблица 3")
    i4 = next(i for i, l in enumerate(t) if l.strip() == "Таблица 4")
    A = {"table2": parse(t[i2:i2 + 25]), "table3": parse(t[i3:i4]),
         "table4": parse(t[i4:i4 + 10], table4=True)}
    o51 = open(r51, encoding="utf-8").read().splitlines()
    o59 = open(r59, encoding="utf-8").read().splitlines()
    # OCR row for 12.43822 reads '13.27315,3.308455' (the page prints '[3.27315,3.308455', no closing bracket)
    B2 = []
    for l in o51:
        l = norm(l).replace("13.27315,3.308455", "[3.27315,3.308455")
        m = re.match(r"^(\d+\.\d{5}) (-?\d\.\d+) (\d\.\d+) (\d/\d) (\S+) (\d\.\d+[лcп])(?: ([+-]))?$", l)
        if m:
            g = list(m.groups())
            if g[6] is None:
                g[6] = EYE_UST.get(g[0], "MISSING")
            B2.append(g)
    B3, B4 = [], []
    for l in o59:
        l = norm(l)
        m = re.match(r"^(\d+\.\d{5}) (-?\d\.\d+) (\d\.\d+) (\d/\d) (\S+) (\d\.\d+[лcп])(?: ([+-]))?$", l)
        m4 = re.match(r"^(0\.\d+) (\d+\.\d{5}) (-?\d\.\d+) (\d\.\d+) (\[\S+\]) (\d\.\d+[лcп]?)$", l)
        if m:
            g = list(m.groups())
            if g[6] is None:
                g[6] = EYE_UST.get(g[0], "MISSING")
            B3.append(g)
        elif m4:
            B4.append(list(m4.groups()))
    B = {"table2": B2, "table3": B3, "table4": B4}
    h23 = ["A", "E", "absJ", "omega", "Imin_Imax", "fig", "ust"]
    h4 = ["m1", "A", "E", "absJ", "Imin_Imax", "fig"]
    for k in A:
        h = h4 if k == "table4" else h23
        for tag, src in (("reading_a_textlayer", A), ("reading_b_ocr", B)):
            with open(os.path.join(outdir, f"{k}_{tag}.csv"), "w", newline="") as f:
                w = csv.writer(f, lineterminator="\n")
                w.writerow(["row"] + h)
                for n, r in enumerate(src[k], 1):
                    w.writerow([n] + r)
        print(k, len(A[k]), "rows A,", len(B[k]), "rows B")


if __name__ == "__main__":
    main(*sys.argv[1:5])
