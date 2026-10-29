#!/usr/bin/env python3
"""Evaluate Part II's printed closed-form Q (p. 1932) at the Fig. 7 parameters, to see whether it reproduces the dashed curve.
Fig. 7 (read off the page image): e_in = 0.6, m1 = 1.5, m2 = m3 = 0.5, e_out = 0.5; dashed curve about 2.6 at cos i = -1 rising to
about 4.35 at cos i = +1 (the dotted Mardling curve is a step curve around 3.5-4.3).
usage: python3 -I check_formula.py
"""
import math


def q(N, cosi, m1=1.5, m3=0.5, m2=0.5, e_in=0.6, e_out=0.5):
    return (3.7 * math.sqrt(N) ** (1 / 6) * (1 + m3 / (m1 + m2)) ** (1 / 3) * (1 - 0.5 * e_in**2) ** (1 / 6)
            * (1 + cosi) ** (1 / 3) * (1 - e_out) ** (-1 / 6))


print("N, Q(cos i = -0.5, 0, +0.5, +1)")
for N in (1, 10, 100, 1000, 10000):
    print(N, [round(q(N, c), 3) for c in (-0.5, 0.0, 0.5, 1.0)])
print("Q(cos i = -1) = 0 for every N, which the paper says cannot be right.")
