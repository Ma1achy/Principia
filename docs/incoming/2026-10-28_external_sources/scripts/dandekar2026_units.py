"""Dandekar et al. 2026: an independent check of Table 1 and its conversion from G = 4 pi^2 to G = 1.

Usage: python3 -I dandekar2026_units.py <table1_reconciled.csv> <Dandekar_2026_Table1_G1_converted.json (ChatGPT)>
The paper (§3.1): "We adopt units such that G = 4π², mass is measured in solar masses (M⊙), and distance is measured in
astronomical units (AU), implying that time is measured in years." The unit map to G = 1 keeps mass (M⊙) and length
(AU) and changes only the time unit: G scales as (time unit)², so G = 1 needs the time unit yr/(2π). Hence
t[G=1] = 2π t[yr], v[G=1] = v/(2π), E[G=1] = E/(4π²), L[G=1] = L/(2π), and any angular momentum such as L0 or L_F,c
likewise /(2π).
Check 1: E and L recomputed from eqs. (28)-(29) at G = 4π² (a = 5 AU, d = 100 AU, the single body the lightest, as §3.1
states), against the printed values. L0 = sqrt(k_B/(-2E)), k_B = mu alpha^2 = G² m_a³ m_b³/(m_a + m_b) for each of the
three pairs; the column order (1), (2), (3) is found by matching.
Check 2: our G = 1 values against ChatGPT's converted file.
"""
import csv
import itertools
import json
import math
import sys


def main():
    G = 4 * math.pi ** 2
    a, d = 5.0, 100.0
    cg = json.load(open(sys.argv[2], encoding="utf-8"))["rows"]
    worst_cg = 0.0
    for r, c in zip(csv.DictReader(open(sys.argv[1], encoding="utf-8")), cg):
        m = [float(x) for x in r["mass_set"].split(",")]
        s = min(range(3), key=lambda i: m[i])
        ma, mb = [m[i] for i in range(3) if i != s]
        ms = m[s]
        E = -G * ma * mb / (2 * a) - G * ms * (ma + mb) / d
        L = math.sqrt(G) * ma * mb * math.sqrt(a / (ma + mb))
        L0 = []
        for i in range(3):
            p, q = [m[j] for j in range(3) if j != i]
            kB = G ** 2 * p ** 3 * q ** 3 / (p + q)
            L0.append(math.sqrt(kB / (-2 * E)))
        printed = [float(r[k]) for k in ("E", "L", "L0_1", "L0_2", "L0_3")]
        print(f"{r['mass_set']:>13}: E ours {E:10.2f} printed {printed[0]:10.2f} | L ours {L:8.2f} printed "
              f"{printed[1]:8.2f} | L0 ours (pair without body 1, 2, 3) {L0[0]:7.2f} {L0[1]:7.2f} {L0[2]:7.2f} "
              f"printed {printed[2]:7.2f} {printed[3]:7.2f} {printed[4]:7.2f}")
        conv = [printed[0] / G, printed[1] / (2 * math.pi)] + [x / (2 * math.pi) for x in printed[2:]]
        theirs = [c["E_G1"], c["L_G1"]] + c["L0_G1"]
        worst_cg = max(worst_cg, max(abs(x - y) / abs(y) for x, y in zip(conv, theirs)))
        print(f"{'':>13}  G = 1: E {conv[0]:.6f}, L {conv[1]:.6f}, L0 {conv[2]:.6f} {conv[3]:.6f} {conv[4]:.6f}")
    print(f"largest relative difference, our G = 1 values against ChatGPT's converted Table 1: {worst_cg:.2e}")
    print(f"time: 1 yr = 2π = {2 * math.pi:.12f} G=1 units; Table 2's fit range 3000 yr to 1e6 yr = "
          f"{3000 * 2 * math.pi:.6f} to {1e6 * 2 * math.pi:.3f}; B.2's window width 30 = {30 / (2 * math.pi):.6f}")


if __name__ == "__main__":
    main()
