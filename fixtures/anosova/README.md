# Agekyan–Anosova region D (R-592; dd_validation_orbits §1.6)

- `table1.csv`: Table 1 of Agekyan & Anosova (1968), *Soviet Astronomy* 11(6), 1006–1014, pp. 1010–1012, every row,
  the "ΔE/E > 0.05" rows and the rows printing "—" included, each with its page. Columns: `page`, `N`, `xi` (column 2),
  `eta_I` (column 3 as paper I prints it), `eta_II` (paper II's Table 1, *Astrophysics* 4(1), 11–16 (1968), where it
  differs), `eta_used` and `eta_source` (paper I's except N = 88, R-592), `E_col8_e7` (column 8), `M_col11_e4`
  (column 11), `outcome` (column 4, as text) and `note` (why the η used is used; broken glyphs read; paper II's `ξ`
  on N = 35, 49, 54 and 65, where it differs, paper I's kept). "—" is printed "—". Transcribed twice, independently,
  from the scan at 600 and 450 dpi, and the two diffed: no difference.
- `region_d.json`: the definition (pp. 1007–1008: eqs. 3, 4 and 8, `G`, the masses and `A₁`, `A₂`) and the printed `M`
  column's exception list, N = 16, 23, 26, 32, 34, each with its printed and computed values (R-592).
- `exact_rest.json`: the exact-rest control (R-592, item 3): the same configurations, velocities exactly 0, `G = 1`.
- `col8_comparison.csv`: column 8 beside eq. 4 on every row, recorded, not asserted (RQ-570, open). Regenerated and
  checked by `crates/validation/tests/literature_import.rs`.
