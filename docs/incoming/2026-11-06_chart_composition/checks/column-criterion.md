# Criterion for the per-column cost, written before measuring (6 Nov 2026)

Stand-in: the kernel's own wrapper loop, f32, native CPU, one thread, no GPU. Region D at rest, W = 64 columns along eta = 0.3,
T = 13, dt_macro = 1e-3, H = 1080 rows on the macro-step grid. A column marches once and copies the SimStateBase into a row slot
as it crosses each row.
C1. Correct: every row's r, p, mean_y, C_ty, total_substeps equals, bitwise, a run with horizon_steps = that row's step.
C2. Overhead: column-with-snapshots <= 1.05 x the same column with no snapshots, median of 5 interleaved passes.
C3. Saving: H normal renders of a column, each from t = 0 (the naive reading) cost >= H/4 x one column.
C4. Report ms per column and per row. GPU not measured: state it.
