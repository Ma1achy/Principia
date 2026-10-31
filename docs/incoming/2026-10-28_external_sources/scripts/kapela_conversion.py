"""Kapela & Simó 2007 and 2017: an independent re-check of ChatGPT's conversion to Cartesian coordinates.

Usage: python3 -I kapela_conversion.py <benchmarks.json (ChatGPT)> <3stability.txt> <eight.txt> <eight_488.txt>
                                       <eight_148.txt> [<out.csv>]
Nothing is integrated. Arithmetic: mpmath at 90 digits. Masses 1, G = 1 (both papers).

KS2007 (Nonlinearity 20, 1241; preprint §4, eq. 4.3 and the text after Theorem 4.5): the centre of mass is fixed at the
origin and body N removed, q_N = -sum q_i, p_N = -sum p_i; the reduced point is z = (x1, vy1, x2, y2, vx2, vy2) with
c = (y1, vx1) = (0, 0). So r1 = (x1, 0), v1 = (0, vy1), r2 = (x2, y2), v2 = (vx2, vy2), r3 = -r1 - r2, v3 = -v1 - v2.
The nominal point is the midpoint of the archive's interval vector K (results.zip, 3stability.txt), as ChatGPT chose.

KS2017 (Nonlinearity 30, 965; arXiv:1105.3235 §3.1; archive KAMstability, BasicEightMap.h::getExpansion and
energy()): Whittaker-reduced coordinates (q1, q2, q3, p1, p2, p3) with Hamiltonian
  H = p1^2 + p2^2 + p3^2 + p1 p2 - (p3/q1)(q2 p3 - q3 p2 - M) + (q2 p3 - q3 p2 - M)^2 / q1^2
      - 1/q1 - 1/sqrt(q2^2 + q3^2) - 1/sqrt((q1 - q2)^2 + q3^2),
the Poincaré section q3 = 0, the archive's four section coordinates (q1, q2, p1, p2), and p3 from H = h by the
archive's root, (-B + sqrt(B^2 - 4AC)) / (2A). Our derivation of the Cartesian state (worked in the README): with
Q1 = r1 - r3 = q1 e(theta), Q2 = r2 - r3 = q2 e(theta) + q3 e_perp(theta), the conjugate momenta are p1 = P1.e,
p2 = P2.e, p3 = P2.e_perp, p_theta = M, and P1.e_perp = (M - q2 p3 + q3 p2)/q1; with unit masses v1 = P1, v2 = P2,
v3 = -P1 - P2. At theta = 0 (our gauge): r1 = ((2 q1 - q2)/3, -q3/3), r2 = ((2 q2 - q1)/3, 2 q3/3),
r3 = (-(q1 + q2)/3, -q3/3), v1 = (p1, s), v2 = (p2, p3), v3 = (-p1 - p2, -s - p3), s = (M - q2 p3 + q3 p2)/q1.
Expanding H with this state reproduces the archive's H (checked numerically below: E(state) = h, L(state) = M).
The nominal point is the archive's "Approximated initial conditions" x0; h is the midpoint of the archive's h
interval (-1.292970857122 exactly); M is the archive's param.

Prints, per record, the largest |ours - ChatGPT| over the 12 components, and E, L, centre of mass and momentum of our
state. Optionally writes our states to out.csv (12 components, 40 significant digits).
"""
import csv
import json
import re
import sys

import mpmath as mp

mp.mp.dps = 90


def nums(s):
    return [mp.mpf(x) for x in re.findall(r"[-+]?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?", s)]


def field(text, name):
    m = re.search(r"^" + re.escape(name) + r"\s*=\s*(.*)$", text, re.M)
    return m.group(1)


def invariants(st):
    r = [(st[0], st[1]), (st[2], st[3]), (st[4], st[5])]
    v = [(st[6], st[7]), (st[8], st[9]), (st[10], st[11])]
    T = sum(a * a + b * b for a, b in v) / 2
    U = sum(1 / mp.sqrt((r[i][0] - r[j][0]) ** 2 + (r[i][1] - r[j][1]) ** 2) for i in range(3) for j in range(i + 1, 3))
    L = sum(x * vy - y * vx for (x, y), (vx, vy) in zip(r, v))
    cm = max(abs(sum(p[0] for p in r)), abs(sum(p[1] for p in r)))
    mom = max(abs(sum(p[0] for p in v)), abs(sum(p[1] for p in v)))
    return T - U, L, cm, mom


def ks2007(text):
    m = re.search(r"vector K\s*\n(\{.*\})", text)
    ends = nums(m.group(1))
    mid = [(ends[2 * i] + ends[2 * i + 1]) / 2 for i in range(6)]
    x1, vy1, x2, y2, vx2, vy2 = mid
    return [x1, 0, x2, y2, -x1 - x2, -y2, 0, vy1, vx2, vy2, -vx2, -vy1 - vy2]


def ks2017(text):
    h_lo, h_hi = nums(field(text, "h"))
    M = nums(field(text, "param"))[0]
    h = (h_lo + h_hi) / 2
    q1, q2, p1, p2 = nums(field(text, "x0"))
    q3 = mp.mpf(0)
    A = 1 - q2 / q1 + (q2 / q1) ** 2
    B = M / q1 - 2 * M * q2 / q1 ** 2
    C = p1 ** 2 + p2 ** 2 + p1 * p2 + (M / q1) ** 2 - 1 / q1 - 1 / abs(q2) - 1 / abs(q1 - q2) - h
    p3 = (-B + mp.sqrt(B * B - 4 * A * C)) / (2 * A)
    s = (M - q2 * p3 + q3 * p2) / q1
    H = (p1 ** 2 + p2 ** 2 + p3 ** 2 + p1 * p2 - (p3 / q1) * (q2 * p3 - q3 * p2 - M) + (q2 * p3 - q3 * p2 - M) ** 2 / q1 ** 2
         - 1 / q1 - 1 / mp.sqrt(q2 ** 2 + q3 ** 2) - 1 / mp.sqrt((q1 - q2) ** 2 + q3 ** 2))
    st = [(2 * q1 - q2) / 3, -q3 / 3, (2 * q2 - q1) / 3, 2 * q3 / 3, -(q1 + q2) / 3, -q3 / 3,
          p1, s, p2, p3, -p1 - p2, -s - p3]
    return st, h, M, H


def main():
    bj = json.load(open(sys.argv[1], encoding="utf-8"))["benchmarks"]
    cg = {b["id"]: [mp.mpf(x) for x in b["cartesian_nominal"]] for b in bj}
    ours = {}
    t07 = open(sys.argv[2], encoding="utf-8").read()
    ours["KS2007_archive_stability"] = (ks2007(t07), None, mp.mpf(0))
    ours["KS2007_paper_Table1"] = ours["KS2007_archive_stability"]
    for rid, path in (("KS2017_archive_eight", sys.argv[3]), ("KS2017_archive_eight_488", sys.argv[4]),
                      ("KS2017_archive_eight_148", sys.argv[5])):
        st, h, M, H = ks2017(open(path, encoding="utf-8").read())
        ours[rid] = (st, h, M)
        print(f"{rid}: h = {mp.nstr(h, 20)}, M = {mp.nstr(M, 20)}, H(reduced state) - h = {mp.nstr(H - h, 3)}")
    worst = (mp.mpf(0), None)
    names = ["x1", "y1", "x2", "y2", "x3", "y3", "vx1", "vy1", "vx2", "vy2", "vx3", "vy3"]
    for rid in [b["id"] for b in bj]:
        st, h, M = ours[rid]
        diffs = [abs(a - b) for a, b in zip(st, cg[rid])]
        k = max(range(12), key=lambda i: diffs[i])
        E, L, cm, mom = invariants(st)
        line = (f"{rid}: max |ours - ChatGPT| = {mp.nstr(diffs[k], 3)} ({names[k]}); our E = {mp.nstr(E, 25)}, "
                f"L = {mp.nstr(L, 5)}, |centre of mass| = {mp.nstr(cm, 3)}, |momentum| = {mp.nstr(mom, 3)}")
        if h is not None:
            line += f"; E - h = {mp.nstr(E - h, 3)}, L - M = {mp.nstr(L - M, 3)}"
        print(line)
        if diffs[k] > worst[0]:
            worst = (diffs[k], f"{rid} {names[k]}")
    print(f"largest disagreement over all {len(bj)} records x 12 components: {mp.nstr(worst[0], 3)} ({worst[1]})")
    if len(sys.argv) > 6:
        with open(sys.argv[6], "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f, lineterminator="\n")
            w.writerow(["id"] + names)
            for rid in [b["id"] for b in bj]:
                w.writerow([rid] + [mp.nstr(x, 40) for x in ours[rid][0]])


if __name__ == "__main__":
    main()
