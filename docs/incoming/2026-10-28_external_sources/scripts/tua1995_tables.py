"""Reading A of Tanikawa, Umehara & Abe (1995) Tables IIa, IIb, IIIa, IIIb, IIIc and IV from Vision OCR of the scans.

Usage: python3 -I tua1995_tables.py <table: 2a|2b|3a|3b|3c|4> <ocr_grid --raw rows of the page> <out.csv>

The PDF is a 1995 scan whose own text layer is poor; it is never read. Reading A is Vision OCR of the 300-dpi page
image (ocr.swift, then ocr_grid.py --minus --raw); reading B is a reading by eye of 1500-pixel-wide crops of the
same pages (the *_reading_b_eye.csv files). Here each OCR line that starts with a table row is split into cells:
  Tables IIa/IIb: x y theta Time v/h phi dx/dy;  Tables IIIa-c: n_min x y theta Time v/h phi;  Table IV: n_min x y alpha.
A cell the OCR did not deliver in a recognisable form is left empty or kept as the OCR's token (and so counts as a disagreement with B). OCR
spellings mapped: 'V' -> 'v'; '10~7', "10-'" -> '10^-7' (stars kept). No digit is altered.
"""
import csv
import re
import sys

DEC = re.compile(r"^\d\.\d+$")


def dxdy(tok):
    m = re.match(r"^10[-~'](\d+)(\**)$", tok)
    return f"10^-{m.group(1)}{m.group(2)}" if m else ""


def main(table, src, out):
    rows = []
    for line in open(src, encoding="utf-8"):
        t = line.split()
        if not t:
            continue
        nmin = ""
        if table in ("3a", "3b", "3c", "4") and re.match(r"^\d{1,2}$", t[0]):
            nmin, t = t[0], t[1:]
        if table == "4":
            if len(t) >= 2 and all(DEC.match(x) or x == "0" for x in t[:3]) and len(t) in (2, 3):
                if len(t) == 2:
                    x, y, a = "", t[0], t[1]
                else:
                    x, y, a = t[:3]
                rows.append(dict(nmin=nmin, x=x, y=y, alpha=a))
            continue
        # a row starts with x and y (7 decimals); theta and Time are the tokens before the v/h, phi or dx/dy
        # marks; if the OCR gives other than two tokens there, both cells are left empty
        if len(t) < 2 or not (re.match(r"^\d\.\d{7}$", t[0]) and re.match(r"^\d\.\d{7}$", t[1])):
            continue
        x, y, t = t[0], t[1], t[2:]
        mid = []
        while t and not (t[0] in ("v", "V", "h", "+-", "-+") or t[0].startswith("10")):
            mid.append(t.pop(0))
        nums = [x, y] + (mid if len(mid) == 2 else ["", ""])
        vh = phi = dd = ""
        for tok in t:
            if tok in ("v", "V", "h"):
                vh = tok.lower()
            elif tok in ("+-", "-+"):
                phi = tok
            elif dxdy(tok):
                dd = dxdy(tok)
        r = dict(nmin=nmin, x=nums[0], y=nums[1], theta=nums[2], time=nums[3], vh=vh, phi=phi)
        if table in ("2a", "2b"):
            r["dxdy"] = dd
            r.pop("nmin")
        rows.append(r)
    if table in ("2a", "2b"):
        fields = ["row", "x", "y", "theta", "time", "vh", "phi", "dxdy"]
    elif table == "4":
        fields = ["row", "nmin", "x", "y", "alpha"]
    else:
        fields = ["row", "nmin", "x", "y", "theta", "time", "vh", "phi"]
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, lineterminator="\n")
        w.writeheader()
        for i, r in enumerate(rows, 1):
            w.writerow(dict(row=i, **r))
    print(table, len(rows), "rows")


if __name__ == "__main__":
    main(*sys.argv[1:4])
