"""Reading A of Titov (2024) Appendix B: parse the PDF's own text layer.

Usage: python3 -I titov_textlayer_reading_a.py <pdftotext -layout output> <out.csv>

Reading A is the text layer (as for Tanikawa et al. 2019, where the owner
counted the text layer as one reading). Reading B is made separately from the
rendered page images (Vision OCR plus eye). Values are kept as printed strings.
Masses come from the section prose: B.2 orbits 1-13 m=(0.95,0.95,1.1);
B.2 orbits 14-16 m=(1.05,1.05,0.9); B.3 (Table 3) m=(0.99,1.01,1.0);
B.3 Table 4 sweep orbits 2-5: m3=1.0, m1 as printed, m2 = 3 - m3 - m1.
"""
import csv
import re
import sys
import unicodedata
from decimal import Decimal

NUM = r"[−\-+]?\d+(?:\.\d+)?"


def norm(s):
    return s.replace("−", "-").lstrip("+")


def main(src, out):
    text = unicodedata.normalize("NFC", open(src, encoding="utf-8").read())
    # drop private-use glyphs (the large brackets of the IC matrices)
    text = "".join(c for c in text if unicodedata.category(c) != "Co")
    lines = text.splitlines()
    start = next(i for i, l in enumerate(lines) if l.strip() == "Приложение B")
    lines = lines[start:]
    rows = []
    section = None
    cur = None
    sweep = False
    for i, l in enumerate(lines):
        s = l.strip()
        if s.startswith("B.2 "):
            section = "B.2"
        elif s.startswith("B.3 "):
            section = "B.3"
        elif "Пять орбит табл. 4" in s:
            sweep = True
        m = re.match(r"^(\d+)\.\s*A\s*=\s*(" + NUM + r").*?ω\s*=\s*(\d+/\d+)", s)
        m2 = re.match(r"^(\d+)\.\s*m1\s*=\s*(" + NUM + r"),\s*A\s*=\s*(" + NUM + r")", s)
        if m and not sweep:
            n = int(m.group(1))
            if section == "B.2":
                masses = ("0.95", "0.95", "1.1") if n <= 13 else ("1.05", "1.05", "0.9")
                tag = f"B2-{n:02d}"
            else:
                masses = ("0.99", "1.01", "1.0")
                tag = f"B3-{n:02d}"
            cur = dict(id=tag, section=section, n=n, m1=masses[0], m2=masses[1],
                       m3=masses[2], A=norm(m.group(2)), omega=m.group(3))
            # omega may be on this line; figure ref may wrap
        elif m2 and sweep:
            n = int(m2.group(1))
            m1 = norm(m2.group(2))
            m2v = str(Decimal(2) - Decimal(m1))
            cur = dict(id=f"T4-{n:02d}", section="B.3 Table 4", n=n, m1=m1,
                       m2=m2v, m3="1.0", A=norm(m2.group(3)), omega="1/2")
        elif re.match(r"^(\d+)\.\s*A\s*=", s):
            raise SystemExit(f"unparsed orbit header: {s}")
        mej = re.search(r"E\s*=\s*(" + NUM + r"),\s*J\s*=\s*(" + NUM + r")", s)
        if mej and cur is not None:
            cur["E"] = norm(mej.group(1))
            cur["J"] = norm(mej.group(2))
        mx = re.match(r"^x\s+(" + NUM + r")\s+x\s+(" + NUM + r")\s+x\s+(" + NUM + r")$", s)
        if mx and cur is not None:
            cur["x1"], cur["x2"], cur["x3"] = (norm(g) for g in mx.groups())
        my = re.match(r"^ẏ1\s+(" + NUM + r")\s+ẏ2\s+(" + NUM + r")\s+ẏ3\s+(" + NUM + r")$", s)
        if my and cur is not None:
            cur["vy1"], cur["vy2"], cur["vy3"] = (norm(g) for g in my.groups())
            # y and xdot rows: check the zeros printed in the block
            block = "\n".join(lines[i - 8:i])
            zeros = re.findall(r"^\s*[123]\s+0\s+[123]\s+0\s+[123]\s+0\s*$", block, re.M)
            vz = re.findall(r"^ẋ1\s+0\s+ẋ2\s+0\s+ẋ3\s+0\s*$", block, re.M)
            cur["y_all_zero"] = "yes" if zeros else "CHECK"
            cur["vx_all_zero"] = "yes" if vz else "CHECK"
            rows.append(cur)
            cur = dict(cur)  # E/J of next orbit overwrite
    fields = ["id", "section", "n", "m1", "m2", "m3", "A", "omega", "E", "J",
              "x1", "x2", "x3", "vy1", "vy2", "vy3", "y_all_zero", "vx_all_zero"]
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, extrasaction="ignore")
        w.writeheader()
        for r in rows:
            w.writerow(r)
    print(len(rows), "rows")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
