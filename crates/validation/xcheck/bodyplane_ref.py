#!/usr/bin/env python3
"""The body plane's fixture, recorded from the Python reference (TASK-M3-36).

Places the moved body with the reference's own `tb.burrau_grid` (numpy.linspace nodes, nominal copies only, `ens=0`)
and evaluates the reference's own algebra on each state: `tb.energy`, `tb.pair_dists`, `tb_ftle.inertia` and
`refine_test.shape_vec`, exactly as `tools/xcheck/dump_ref.py`'s algebra case does. Nothing of the reference is
reimplemented here, and nothing under docs/reference/ is edited.

Run from the workspace root (CI has no numpy, so the output is committed and the Rust side reads it):
    python3 -I crates/validation/xcheck/bodyplane_ref.py --out fixtures/golden/bodyplane/reference.tsv
"""
import argparse
import os
import platform
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "docs", "reference", "prin-rs", "reference"))

import numpy as np  # noqa: E402
import tb  # noqa: E402
import tb_ftle  # noqa: E402
import refine_test  # noqa: E402

# The windows: the five Burrau regions of fixtures/slices.toml at a 9 x 9 grid, and the Python cross-check's own
# 3 x 3 near-field window (tools/xcheck/cases.py's AZ cases). (name, body, cx, cy, half, n)
CASES = [
    ("near_field", 0, 1.0, 3.0, 0.05, 9),
    ("mid_field", 0, 1.0, 6.0, 0.05, 9),
    ("far", 0, 1.0, 13.0, 0.05, 9),
    ("body2_core", 2, 1.0, -1.0, 0.05, 9),
    ("deep_interior", 0, 0.0, 0.0, 0.05, 9),
    ("xcheck_near_field", 0, 1.0, 3.0, 0.05, 3),
]

COLUMNS = "case,i,j,x,y,energy,d01,d02,d12,inertia,hyperradius,n0,n1,n2"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    rows = []
    for k, (_name, body, cx, cy, half, n) in enumerate(CASES):
        r0, v0, _gid, _, _hx = tb.burrau_grid(n, n, cx, cy, half, body=body, ens=0)
        assert not v0.any(), "the reference's body plane starts at rest"
        energy = tb.energy(r0, v0, 0.0)
        pd = tb.pair_dists(r0)
        inertia = tb_ftle.inertia(r0)
        hyper = np.sqrt(inertia / tb.M.sum())
        shape = refine_test.shape_vec(r0)
        for idx in range(r0.shape[0]):
            # burrau_grid's row order: meshgrid(indexing='xy') then ravel, so idx = j*n + i, x fastest.
            i, j = idx % n, idx // n
            x, y = r0[idx, body, 0], r0[idx, body, 1]
            vals = [x, y, energy[idx], pd[idx, 0], pd[idx, 1], pd[idx, 2], inertia[idx], hyper[idx],
                    shape[idx, 0], shape[idx, 1], shape[idx, 2]]
            rows.append(f"{k}\t{i}\t{j}\t" + "\t".join("%.17e" % v for v in vals))
    os.makedirs(os.path.dirname(a.out) or ".", exist_ok=True)
    with open(a.out, "w") as f:
        f.write("# body plane fixture, recorded from docs/reference/prin-rs/reference (tb.burrau_grid, tb.energy, "
                "tb.pair_dists, tb_ftle.inertia, refine_test.shape_vec) by crates/validation/xcheck/bodyplane_ref.py\n")
        f.write(f"# numpy={np.__version__} python={platform.python_version()} platform={platform.machine()}-"
                f"{platform.system()}\n")
        f.write("# masses=3,4,5 G=1 positions=(1,3),(-2,-1),(1,-1) at rest\n")
        for k, (name, body, cx, cy, half, n) in enumerate(CASES):
            f.write(f"# case {k} name={name} body={body} cx={cx!r} cy={cy!r} half={half!r} n={n}\n")
        f.write(f"# columns={COLUMNS}\n")
        for row in rows:
            f.write(row + "\n")
    print(f"wrote {a.out}: {len(rows)} rows")


if __name__ == "__main__":
    main()
