"""Internal consistency of Tanikawa, Umehara & Abe (1995) Table IV (triple-collision points on Gamma_3).

Usage: python3 -I tua1995_table4_check.py <table4_reconciled.csv> <out.txt>

Gamma_3 is the circular boundary of D, (x + 0.5)^2 + y^2 = 1 (eq. (1): m1 at P(x, y), m2 at A(-0.5, 0),
m3 at B(0.5, 0), so PA = AB = 1 on Gamma_3). For each row this reports the circle residual
(x + 0.5)^2 + y^2 - 1 and the angle PAB = atan2(y, x + 0.5), against the printed alpha ("the vertex angle of
initial isosceles triangle"). Exact decimal arithmetic via mpmath at 30 digits.
"""
import csv
import sys

import mpmath as mp

mp.mp.dps = 30


def main(src, outp):
    out = ["Table IV: circle residual and angle PAB = atan2(y, x + 0.5) against the printed alpha", ""]
    for r in csv.DictReader(open(src)):
        x, y, a = mp.mpf(r["x"]), mp.mpf(r["y"]), mp.mpf(r["alpha"])
        res = (x + mp.mpf("0.5")) ** 2 + y ** 2 - 1
        ang = mp.atan2(y, x + mp.mpf("0.5"))
        out.append(f"n_min {r['nmin']:>2}: x={r['x']} y={r['y']} residual {mp.nstr(res, 3):>10}  "
                   f"PAB {mp.nstr(ang, 8)}  alpha {r['alpha']}  PAB - alpha {mp.nstr(ang - a, 3)}")
    out.append("")
    x9 = mp.mpf("0.4911357")
    y9 = mp.mpf("0.1328464")
    out.append(f"row 9 read with 7 decimals, x = 0.4911357: residual {mp.nstr((x9 + mp.mpf('0.5'))**2 + y9**2 - 1, 3)}, "
               f"PAB {mp.nstr(mp.atan2(y9, x9 + mp.mpf('0.5')), 8)}")
    open(outp, "w").write("\n".join(out) + "\n")
    print("\n".join(out))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
