"""Diff the two transcriptions of each Agekyan-Anosova table, apply the recorded resolutions, write the CSVs,
and print every check against the papers' own summary numbers.

Usage (from this folder): python3 -I build.py ../../../../fixtures/anosova/table1.csv

Sources:
  I  = T. A. Agekyan & Zh. P. Anosova, Soviet Astronomy 11(6), 1006-1014 (1968), Table 1, pp. 1010-1012
       (docs/incoming/2026-10-24_anosova/papers/agekyan_anosova_1968_SvA_11_1006.pdf, an ADS image scan).
  II = the same authors, Astrophysics 4(1), 11-16, doi:10.1007/BF01013153, Table 1 (pp. 12-14) and Table 3 (p. 15)
       (docs/incoming/2026-10-24_anosova/papers/agekyan_anosova_1968_Ap_4_11_part2.pdf).

A and B are independent readings: A from full-page renders at 300 dpi (II) or 400 dpi strips (I); B from separate
column-strip crops (II) or the 1400-pixel page overviews (I). The script diffs them and stops on any difference that
is not in RESOLVED.
"""
import csv
import statistics as st
import sys

# (table, N, column index in the A line, A value, B value) -> value kept, and why.
RESOLVED = {
    ("II.1", 16, 6): ("0.003", "B read the printed glyph literally as 0.903; II p. 16 says column 7 never exceeds "
                               "0.02 and the PDF's own text layer reads 0,003; kept 0.003"),
}


def read(path):
    return [line.split() for line in open(path) if line.strip()]


def diff(table, a_rows, b_rows, a_cols):
    """a_cols maps B's column positions to A's."""
    assert len(a_rows) == len(b_rows), (table, len(a_rows), len(b_rows))
    out = []
    for a, b in zip(a_rows, b_rows):
        n = int(a[0])
        assert int(b[0]) == n, (table, a[0], b[0])
        for jb, ja in enumerate(a_cols):
            if a[ja] != b[jb]:
                key = (table, n, ja)
                if key not in RESOLVED:
                    sys.exit(f"UNRESOLVED {table} N={n} col={ja}: A {a[ja]} B {b[jb]}")
                out.append((key, a[ja], b[jb], RESOLVED[key]))
    return out


def num(s):
    return None if s == "-" else float(s.rstrip("*"))


def main():
    fixture = {int(r["N"]): r for r in csv.DictReader(open(sys.argv[1]))}

    t1a, t1b = read("aa2_table1_A.txt"), read("aa2_table1_B.txt")
    t3a, t3b = read("aa2_table3_A.txt"), read("aa2_table3_B.txt")
    i1a, i1b = read("aa1_table1_cols5-8_A.txt"), read("aa1_table1_col5_B.txt")
    diffs = []
    diffs += diff("II.1", t1a, t1b, [0] + list(range(3, 12)))
    diffs += diff("II.3", t3a, t3b, list(range(0, 12)))
    diffs += diff("I.1", i1a, i1b, [0, 1])
    print("== A against B ==")
    for key, a, b, (kept, why) in diffs:
        print(f"  {key}: A {a}  B {b}  -> {kept}  ({why})")
    print(f"  {len(diffs)} difference(s), all resolved")

    # Apply resolutions to A (the kept values equal A's here, so nothing changes; asserted).
    for (table, n, ja), a, b, (kept, why) in diffs:
        assert kept == a, "a resolution that departs from A must be applied explicitly"

    print("\n== II Table 1 (exact rest) against the paper's own statements ==")
    T = [num(r[3]) for r in t1a]
    print(f"  rows {len(t1a)}; mean T {st.mean(T):.2f} (paper: 110.1); sd {st.stdev(T):.2f} (paper: 125.0)")
    nT = sum(r[3].endswith("*") for r in t1a)
    nP = sum(r[4].endswith("*") for r in t1a)
    both = [r[0] for r in t1a if r[3].endswith("*") and r[4].endswith("*")]
    print(f"  asterisked T {nT} (paper p. 16: 12 conditional decays); asterisked p {nP} (paper: 19); "
          f"both {len(both)} {both} (paper: 6)")
    dE = [abs(float(r[6])) for r in t1a]
    print(f"  |dE/E| max {max(dE)} (paper: does not exceed 0.02); <0.01: {sum(d < 0.01 for d in dE)} (paper 84); "
          f"<0.005: {sum(d < 0.005 for d in dE)} (paper 55); <0.0005: {sum(d < 0.0005 for d in dE)} (paper 13)")
    printed = {(0, 0.1): (15, 2.45), (0.1, 0.2): (35, 0.412), (0.2, 99): (50, 0.205)}
    for (lo, hi), (n_p, m_p) in printed.items():
        sel = [float(r[7]) for r in t1a if lo < num(r[4]) <= hi]
        print(f"  Table 2, {lo} < p <= {hi}: n {len(sel)} (paper {n_p}), mean D'E/-E {st.mean(sel):.3f} "
              f"(paper {m_p})")
    bad = []
    for r in t1a:
        n = int(r[0]); f = fixture[n]
        eta = f["eta_II"] or f["eta_I"]
        if r[2] != eta:
            bad.append(("eta", n))
        if r[1] != f["xi"] and r[1] not in f["note"]:
            bad.append(("xi", n))
    print(f"  xi, eta against fixtures/anosova/table1.csv (II's values where II differs): {len(bad)} mismatches {bad}")

    print("\n== II Table 3 (the 38 near-rest runs I left unfinished) ==")
    dash = sorted(n for n, f in fixture.items() if f["E_col8_e7"] == "—")
    rows3 = sorted(int(r[0]) for r in t3a)
    print(f"  rows {len(rows3)}; same set as I's rows printing '—' in column 8: {rows3 == dash}")
    mdev = [(int(r[0]), r[6], float(fixture[int(r[0])]["eta_used"])) for r in t3a
            if abs(float(r[6]) - float(fixture[int(r[0])]["eta_used"])) > 6e-5]
    print(f"  M*1e4 against eta (M = eta*1e-4 under I's eq. 3): {len(t3a) - len(mdev)} of {len(t3a)} agree to the "
          f"printed 4 decimals; differ: {mdev}")

    print("\n== I Table 1 columns 5-8 ==")
    emis = [r[0] for r in i1a if fixture[int(r[0])]["E_col8_e7"] != r[4]]
    print(f"  rows printing column 8: {len(i1a)}; column 8 against the fixture: {len(emis)} mismatches {emis}")
    TI = [num(r[1]) for r in i1a if r[1] != "-"]
    print(f"  T values {len(TI)} (paper: 59); mean {st.mean(TI):.2f} (paper eq. 12: 61.6); sd {st.stdev(TI):.2f} "
          f"(paper eq. 13: 74.4)")
    T3 = [num(r[1]) for r in t3a]
    comb = TI + T3
    print(f"  I's {len(TI)} + II Table 3's {len(T3)} = {len(comb)} values: mean {st.mean(comb):.2f}, sd "
          f"{st.stdev(comb):.2f} (II p. 16: 80.7 and 83.0, 'for 100 systems')")

    write_csvs(t1a, t3a, i1a)


def write_csvs(t1a, t3a, i1a):
    with open("../agekyan_anosova_II_table1_exact_rest.csv", "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["page", "N", "xi", "eta", "T", "T_conditional", "p", "p_closer_earlier", "minusE_e7",
                    "dE_over_minusE", "dpE_over_minusE", "n1", "n2", "k", "s", "note"])
        for r in t1a:
            n = int(r[0]); page = 12 if n <= 35 else 13 if n <= 70 else 14
            note = RESOLVED[("II.1", 16, 6)][1] if n == 16 else ""
            w.writerow([page, n, r[1], r[2], r[3].rstrip("*"), int(r[3].endswith("*")), r[4].rstrip("*"),
                        int(r[4].endswith("*")), r[5], r[6], r[7], r[8], r[9], r[10], r[11], note])
    with open("../agekyan_anosova_II_table3_near_rest_completed.csv", "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["page", "N", "T", "T_conditional", "p", "p_closer_earlier", "minusE_e7", "dE_over_minusE",
                    "dpE_over_minusE", "M_e4", "dM_over_M_e4", "n1", "n2", "k", "s"])
        for r in t3a:
            w.writerow([15, int(r[0]), r[1].rstrip("*"), int(r[1].endswith("*")), r[2].rstrip("*"),
                        int(r[2].endswith("*")), r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[10], r[11]])
    with open("../agekyan_anosova_I_table1_cols5to8.csv", "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["page", "N", "T", "p", "p_min", "E_e7", "note"])
        for r in i1a:
            n = int(r[0]); page = 1010 if n <= 52 else 1011 if n <= 96 else 1012
            note = ""
            if 32 <= n <= 37:
                note = "column 5 is set about half a line low on rows 32-37; assigned in order to the rows printing column 8"
            if r[1] == "-":
                note = "nominal disintegration with T and p printed as a dash (I p. 1009: last triple approach not recognised)"
            w.writerow([page, n, r[1], r[2], r[3], r[4], note])


if __name__ == "__main__":
    main()
