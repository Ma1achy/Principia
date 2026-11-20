"""freefall_checks.py: arithmetic checks for Tanikawa & Umehara 1998 and Martynova & Orlov 2014.

1. TU 1998 p. 172: the three Fig. 4 points lie on the circle of radius 0.95 about A(-0.5, 0).
2. MO 2014 p. 760: interior grid points of D = {x >= 0, y >= 0, (x + 0.5)^2 + y^2 <= 1} at step
   0.001, boundaries excluded, counted in exact integers, against the printed 306,346.
Pure arithmetic; nothing is integrated.
"""
from decimal import Decimal as D
for p, x, y in (("4a", "0.12075", "0.71915"), ("4b", "0.09863", "0.73766"), ("4c", "0.07000", "0.76000")):
    r = ((D(x) + D("0.5")) ** 2 + D(y) ** 2).sqrt()
    print("TU1998 %s: |P - A| = %.7f (printed circle 0.95; diff %.1e)" % (p, r, r - D("0.95")))
N = 1000
strict = sum(1 for i in range(1, N) for j in range(1, N + 1) if (i + N // 2) ** 2 + j * j < N * N)
le = sum(1 for i in range(1, N) for j in range(1, N + 1) if (i + N // 2) ** 2 + j * j <= N * N)
print("MO2014: interior points, arc excluded %d, arc included %d; printed 306346; difference %d"
      % (strict, le, strict - 306346))
print("MO2014: survivors 26 / 306346 = %.3e; decayed 306320 + 26 = %d" % (26 / 306346, 306320 + 26))
