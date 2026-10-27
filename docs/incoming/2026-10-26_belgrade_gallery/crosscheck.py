"""Cross-check the Belgrade gallery extraction against its published sources.

Usage: python3 -I crosscheck.py <gallery.csv> <initial-conditions.pdf> <hudomal_2015_thesis.pdf> <suvakov_2013_prl.pdf>

Needs `pdftotext` (poppler). Prints four checks:
  1. every labelled sequence orbit (I.n.X ... VIII.n.X) against Hudomal's "New Periodic Three-Body Orbits" table
     (initial-conditions.pdf, on the gallery, 2019): p1, p2, T to the printed digits;
  2. the free-group word of every labelled orbit against its label: n_a = n_b, n_A = n_B, and the label's n equals
     min(n_a, n_A) (Hudomal 2015 §4.1, conventions 1-2);
  3. the 15 named orbits of Šuvakov & Dmitrašinović 2013, Table 1 (reprinted as Hudomal 2015, Table 1): numbers, and
     words compared up to cyclic rotation and the symmetries listed, after free and cyclic reduction;
  4. Hudomal 2015 Tables 16-22 (rescaled to E = -1/2 with x1 = -lambda) against the gallery, rescaled the same way.
"""

import csv
import math
import re
import subprocess
import sys

INV = {"a": "A", "A": "a", "b": "B", "B": "b"}
LABEL = r"(?:VIIa|VIIb|VIII|VII|VI|IVa|IVb|IVc|V|IV|III|II|I)\.\d+\.[A-Z](?:\.[αβ])?"


def text(pdf):
    return subprocess.run(["pdftotext", "-layout", pdf, "-"], capture_output=True, text=True, check=True).stdout


def expand(s):
    s = s.replace(" ", "").replace("×", "")
    while True:
        s = re.sub(r"([abAB])(\d+)", lambda m: m.group(1) * int(m.group(2)), s)
        m = re.search(r"\(([abAB]*)\)(\d*)", s)
        if not m:
            break
        s = s[: m.start()] + m.group(1) * int(m.group(2) or 1) + s[m.end():]
    assert re.fullmatch("[abAB]+", s), s
    return s


def reduce_cyclic(w):
    st = []
    for c in w:
        if st and st[-1] == INV[c]:
            st.pop()
        else:
            st.append(c)
    while len(st) > 1 and st[0] == INV[st[-1]]:
        st = st[1:-1]
    return "".join(st)


def rots(w):
    return {w[i:] + w[:i] for i in range(len(w))}


def variants(w):
    return {
        "cyclic": rots(w),
        "inverse": rots("".join(INV[c] for c in reversed(w))),
        "reverse": rots(w[::-1]),
        "a<->b": rots(w.translate(str.maketrans("abAB", "baBA"))),
        "(a,b)<->(A,B)": rots(w.translate(str.maketrans("abAB", "ABab"))),
    }


# Šuvakov & Dmitrašinović 2013, Table 1, transcribed from the PRL and checked against Hudomal 2015, Table 1.
T1 = {
    "BUTTERFLY I": (0.30689, 0.12551, 6.2356, "(ab)2(AB)2"),
    "BUTTERFLY II": (0.39295, 0.09758, 7.0039, "(ab)2(AB)2"),
    "BUMBLEBEE": (0.18428, 0.58719, 63.5345, "(b2(ABab)2A2(baBA)2ba)(B2(abAB)2a2(BAba)2BA)"),
    "MOTH I": (0.46444, 0.39606, 14.8939, "ba(BAB)ab(ABA)"),
    "MOTH II": (0.43917, 0.45297, 28.6703, "(abAB)2A(baBA)2B"),
    "BUTTERFLY III": (0.40592, 0.23016, 13.8658, "(ab)2(ABA)(ba)2(BAB)"),
    "MOTH III": (0.38344, 0.37736, 25.8406, "(babABA)2a(abaBAB)2b"),
    "GOGGLES": (0.08330, 0.12789, 10.4668, "(ab)2ABBA(ba)2BAAB"),
    "BUTTERFLY IV": (0.350112, 0.07934, 79.4759, "((ab)2(AB)2)6A((ba)2(BA)2)6B"),
    "DRAGONFLY": (0.08058, 0.58884, 21.2710, "(b2(ABabAB))(a2(BAbaBA))"),
    "YARN": (0.55906, 0.34919, 55.5018, "(babABabaBA)3"),
    "YIN-YANG 1a": (0.51394, 0.30474, 17.3284, "(ab)2(ABA)ba(BAB)"),
    "YIN-YANG 1b": (0.28270, 0.32721, 10.9626, "(ab)2(ABA)ba(BAB)"),
    "YIN-YANG 2a": (0.41682, 0.33033, 55.7898, "(abaBAB)3(abaBAbab)(ABAbab)3(AB)2"),
    "YIN-YANG 2b": (0.41734, 0.31310, 54.2076, "(abaBAB)3(abaBAbab)(ABAbab)3(AB)2"),
}


def main(gcsv, icpdf, thesis, prl):
    rows = [r for r in csv.DictReader(open(gcsv, encoding="utf-8")) if r["id"]]
    g = {r["name"].strip(): r for r in rows}
    seq = [k for k in g if re.fullmatch(LABEL, k)]

    print("== 1. Sequence orbits against Hudomal, initial-conditions.pdf ==")
    pdf = {}
    for m in re.finditer(rf"^\s*({LABEL})\s+(-?\d+\.\d+)\s+(-?\d+\.\d+)\s+(\d+\.\d+)", text(icpdf), re.M):
        pdf.setdefault(m.group(1), tuple(map(float, m.groups()[1:])))
    both = [k for k in seq if k in pdf]
    worst = max(
        max(abs(float(g[k]["p1"]) - pdf[k][0]), abs(float(g[k]["p2"]) - pdf[k][1]), abs(float(g[k]["period"]) - pdf[k][2]))
        for k in both
    )
    print(f"pdf rows {len(pdf)}, gallery labelled orbits {len(seq)}, in both {len(both)}; "
          f"gallery only {[k for k in seq if k not in pdf]}, pdf only {[k for k in pdf if k not in g]}")
    print(f"max |difference| over p1, p2, T: {worst:.1e}")

    print("\n== 2. Words against labels ==")
    bad = []
    for k in seq:
        w = g[k]["word"]
        n = int(k.split(".")[1])
        c = {x: w.count(x) for x in "abAB"}
        if not (c["a"] == c["b"] and c["A"] == c["B"] and n == min(c["a"], c["A"])):
            bad.append((k, c))
    print(f"{len(seq) - len(bad)} of {len(seq)} consistent; inconsistent: {bad}")

    print("\n== 3. Šuvakov & Dmitrašinović 2013, Table 1 ==")
    prl_text = text(prl)
    for k, (x, y, T, word) in T1.items():
        r = g[k]
        dn = max(abs(float(r["p1"]) - x), abs(float(r["p2"]) - y), abs(float(r["period"]) - T))
        raw = expand(word)
        red = reduce_cyclic(raw)
        hits = [name for name, v in variants(red).items() if r["word"] in v]
        note = " (published word is not freely reduced)" if len(red) != len(raw) else ""
        print(f"{k:14s} max|d numbers| {dn:.1e}  word: {', '.join(hits) if hits else 'NO MATCH'}{note}")
    assert "moth III" in prl_text

    print("\n== 4. Hudomal 2015, Tables 16-22 (E = -1/2) ==")
    th = {}
    lines = text(thesis).split("\n")
    lab = re.compile(rf"^\s+({LABEL})\s+(-?\d.*)$")
    num = re.compile(r"-?\d+\.\d+(?: \d{1,3}(?=\s|$))*")
    start = next(i for i, l in enumerate(lines) if "Table 16:" in l)
    end = next(i for i, l in enumerate(lines) if "Table 23:" in l)
    for l in lines[start:end]:
        m = lab.match(l)
        if m:
            ns = [float(v.replace(" ", "")) for v in num.findall(m.group(2))]
            if len(ns) >= 4:
                th.setdefault(m.group(1), ns[:4])
    gal = {}
    for k in seq:
        r = g[k]
        lam = -2 * float(r["energy"])
        gal[k] = (float(r["p1"]) / math.sqrt(lam), float(r["p2"]) / math.sqrt(lam), lam, float(r["period"]) * lam**1.5)
    for k, (xd, yd, lt, Tt) in th.items():
        own = gal.get(k)
        d = lambda q: max(abs(q[0] - xd), abs(q[1] - yd), abs(q[2] - lt) / lt, abs(q[3] - Tt) / Tt)
        if own and d(own) < 1e-5:
            continue
        best = min(gal, key=lambda j: d(gal[j]))
        print(f"thesis {k:10s} -> gallery {k if own else '(absent)'} differs by {d(own) if own else float('nan'):.1e}; "
              f"nearest gallery orbit {best} at {d(gal[best]):.1e}")
    print(f"thesis rows {len(th)}; all others agree with the gallery's same label to 1e-5 (printed precision)")


if __name__ == "__main__":
    main(*sys.argv[1:5])
