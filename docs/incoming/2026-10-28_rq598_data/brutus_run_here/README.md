# Brutus, run here: the RQ-598 cost pilot (GG-1 and BR-1)

**These are our runs.** Every number in this folder was computed here, with Boekholt's Brutus as pinned in `INSTALL_RECORD.md`. None of
it is the papers' data, and none of it is a result of the papers. Under the human's ruling of 28 Oct 2026 it is labelled "Brutus, run
here". Whether a self-computed ensemble may score a target or is a cross-check only is still open
(`../README.md` §3); nothing here is offered for scoring.

A pilot, not a scale-up: ε ≥ 10⁻⁶ except one approved point at 10⁻⁸ and 10⁻¹⁰ (29 Oct 2026), 16 GG-1 grid points, 12 BR-1 trajectories (plus tolerance and word-length
probes). `REPORT.md` has the findings and the projections, with measured and estimated numbers marked.

| Path | What |
|---|---|
| `REPORT.md` | the report (also posted on PR #327) |
| `INSTALL_RECORD.md`, `install_record_machine.txt`, `binaries.sha256`, `build.sh`, `build_stock.log`, `stock_lw_test.txt` | what was installed and built, exactly |
| `src/pilot.cpp` | our driver (GG-1 lifetimes with reversibility test; BR-1 trajectories) |
| `gg1/make_grid.py`, `grid_1212.csv`, `make_grid_output.txt` | the 1212-point 1/64 grid on the Agekyan–Anosova map, cross-checked against the Fig. 1 extraction (`data/rq598-gg1-fig1-v2`): identical sets |
| `gg1/select_sample.py`, `sample_16.csv` | the 16-point sample (stratified on the paper's own Fig. 1 lifetimes; used only to choose points) |
| `gg1/run_gg1.sh`, `pilot_eps1e-6_Lw1024.csv`, `gg1_table.md` | the sample run and its raw output |
| `gg1/pilot_fwdonly_final_driver_*.csv` | forward-only re-run with the final driver (identical steps and T) |
| `gg1/run_below_1e6.sh`, `run_below_1e6_pmax.sh`, `below_1e6_*.csv` | the two probes below ε = 10⁻⁶ the human approved on 29 Oct (point (1, 54)); `REPORT.md` § 2b |
| `gg1/run_scaling.sh`, `scaling_gg1*.csv`; `run_pmax_probe.sh`, `pmax_probe_gg1.csv` | ε, L_w and p_max cost probes |
| `gg1/robustness_three_points_Lw128_norev.csv`, `ours_vs_fig1.csv` | the lifetime-definition finding in `REPORT.md` |
| `br1/select_sample.py`, `sample_12.csv`, `run_br1.py`, `results_br1*.csv`, `traj/` (ε = 10⁻⁶, L_w = 128 only), `compare.py`, `compare_to_authors.csv`, `br1_table.md` | BR-1 sample, runs and the point-by-point comparison |
| `summarise.py`, `summary_output.txt`, `projections.py`, `projections_output.txt` | every statistic and projection, regenerable from the CSVs |

**Not committed.** The authors' trajectories (`pgbreen/NVM` @ `a35c378`, `trainex/data/run10–99.dat`, `validex/data/run9910–9999.dat`) have no
licence. The copy used lives in `docs/incoming/_local/nvm_a35c378/` (gitignored). `br1/compare.py` reads it from there; `sample_12.csv` lists
run names, each run's printed first-row coordinates (the initial condition x₂), the authors' row counts, and three values derived from their file (minimum pair separation, its time, distance of x₂ from the singular point).

**Reproduce.** `./build.sh <scratch dir>`, then `gg1/make_grid.py`, `select_sample.py`, `run_gg1.sh`, `br1/run_br1.py`, `compare.py`,
`summarise.py`, `projections.py` (arguments in each script's header). The `traj/` files for ε ≠ 10⁻⁶ are not committed; `run_br1.py` regenerates them.
Wall times are for this machine (a 2.8 GHz Xeon vCPU, `-O1`, three jobs side by side on four cores); the Mac will differ. CPU seconds are in the CSVs.
