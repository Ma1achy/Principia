"""Reading B of Titov (2024) Appendix B from Vision OCR of the rendered pages.

Usage: python3 -I titov_ocr_reading_b.py <out.csv> <page1.tsv> [<page2.tsv> ...]

Each .tsv is the output of ocr.swift for one 300-dpi page image, given in page
order. Orbit headers, E/J lines and the six non-zero initial conditions
(x1, x2, x3 in the top row of the block, vy1, vy2, vy3 in the bottom row) are
taken from the OCR tokens only; the text layer is never read. Cells the OCR
did not deliver are written as MISSING, to be filled by eye.
Ids come from the section (B.2 until "Линейная симметрия", B.3 until "Пять орбит",
then the Table 4 sweep) and the header number the OCR read.
"""
import csv
import re
import sys

IC = re.compile(r"^(-?\d\.\d{7})\W*$")


def norm(s):
    for ch in "−–—":
        s = s.replace(ch, "-")
    return s.replace("А", "A").replace("Е", "E")


def main(out, pages):
    toks = []
    for pi, p in enumerate(pages):
        page = []
        for line in open(p, encoding="utf-8"):
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 4:
                continue
            y, x0, x1, t = float(parts[0]), float(parts[1]), float(parts[2]), norm(parts[3])
            page.append((pi, -y, x0, t))
        page.sort()
        toks.extend(page)
    rows = []
    cur = None
    in_ic = False
    ics = []

    def close_ic():
        nonlocal ics
        if cur is None:
            return
        if len(ics) == 6:
            ics.sort(key=lambda t: (t[0], t[1]))
            top, bot = sorted(ics[:3], key=lambda t: t[2]), sorted(ics[3:], key=lambda t: t[2])
            cur.update(x1=top[0][3], x2=top[1][3], x3=top[2][3],
                       vy1=bot[0][3], vy2=bot[1][3], vy3=bot[2][3])
            cur["ic_tokens"] = 6
        else:
            cur["ic_tokens"] = len(ics)
            cur["ic_raw"] = " ".join(t[3] for t in sorted(ics))
        ics = []

    started = False
    sec = "B2"
    for pi, ny, x0, t in toks:
        if "Приложение B" in t or "Хореографии 2-1" in t:
            started = True
        if not started:
            continue
        if "Линейная симметрия" in t:
            sec = "B3"
        if "Пять орбит" in t:
            sec = "T4"
        mh = re.match(r"^(\d+)\.\s*A\s*=\s*([\d.]+)", t)
        ms = re.match(r"^(\d+)\.\s*[mт]1\s*=\s*([\d.]+),\s*A\s*=\s*([\d.]+)", t)
        if mh or ms:
            if in_ic:
                close_ic()
            in_ic = False
            n = int((mh or ms).group(1))
            cur = dict(id=f"{sec}-{n:02d}", n_ocr=str(n))
            if ms:
                cur.update(m1=ms.group(2), A=ms.group(3).rstrip("."))
            else:
                cur["A"] = mh.group(2).rstrip(",")
                mw = re.search(r"[wω]\s*=\s*(\d+/\d+)", t)
                cur["omega"] = mw.group(1) if mw else "MISSING"
            rows.append(cur)
            continue
        mej = re.search(r"E\s*=\s*(-?\s*[\d.]+),?\s*J\s*=\s*(-?[\d.]+)", t)
        if mej and cur is not None:
            cur["E"] = mej.group(1).replace(" ", "").rstrip(".")
            cur["J"] = mej.group(2).rstrip(".")
        if "Начальные условия" in t:
            in_ic = True
            continue
        if in_ic:
            m = IC.match(t)
            if m:
                ics.append((pi, ny, x0, m.group(1)))
                if len(ics) == 6:
                    close_ic()
                    in_ic = False
    fields = ["id", "n_ocr", "m1", "A", "omega", "E", "J", "x1", "x2", "x3",
              "vy1", "vy2", "vy3", "ic_tokens", "ic_raw"]
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, restval="MISSING")
        w.writeheader()
        w.writerows(rows)
    print(len(rows), "orbits;", sum(1 for r in rows if r.get("ic_tokens") != 6), "with IC tokens != 6")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])
