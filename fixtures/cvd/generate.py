#!/usr/bin/env python3
"""Generates fixtures/cvd/goldens.json, TASK-M7-20's colour-vision golden values (REQ-COL-042, REQ-COL-045).

The reference implementation is DaltonLens-Python at the commit R-383 pins (dd_colouring §3.8): its
Simulator_Vienot1999 for protan and deutan and its Simulator_Brettel1997 for tritan, both on their default LMS model,
LMSModel_sRGB_SmithPokorny75, with Brettel's defaults (use_white_as_neutral=True, use_vischeck_anchors=False). The
goldens are taken in linear sRGB, through each simulator's linear-RGB path, _simulate_dichromacy_linear_rgb, at full
dichromacy (severity 1), never through its 8-bit path (R-383).

Run it on a read-only clone of the pinned commit, with numpy and nothing else installed:

    git clone https://github.com/DaltonLens/DaltonLens-Python.git <clone>
    git -C <clone> checkout 3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d
    python3 fixtures/cvd/generate.py <clone>

It refuses a clone at another commit or with local changes. It imports only the package's convert, simulate and utils
modules, which need numpy alone; the package's __init__ (its command-line main, which needs Pillow) is not run.
"""

import contextlib
import io
import json
import subprocess
import sys
import types
from pathlib import Path

REPOSITORY = "https://github.com/DaltonLens/DaltonLens-Python"
COMMIT = "3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d"
OUT = Path(__file__).resolve().parent / "goldens.json"

# The test colour set: the 8-bit sRGB lattice of step 51 (0, 51, ..., 255 on each channel, 216 colours), then the
# hatch's violet and cyan (render contract Part 5) and the Okabe-Ito palette (render contract Part 5, dbg_cat).
LATTICE = [0, 51, 102, 153, 204, 255]
EXTRA = [
    (0x9B, 0x00, 0xFF), (0x48, 0xFF, 0xFF),
    (0x00, 0x00, 0x00), (0xE6, 0x9F, 0x00), (0x56, 0xB4, 0xE9), (0x00, 0x9E, 0x73),
    (0xF0, 0xE4, 0x42), (0x00, 0x72, 0xB2), (0xD5, 0x5E, 0x00), (0xCC, 0x79, 0xA7),
]


def git(clone, *args):
    return subprocess.run(["git", "-C", str(clone), *args], check=True, capture_output=True, text=True).stdout.strip()


def load(clone):
    """The pinned package's convert and simulate modules, imported without running its __init__."""
    package = types.ModuleType("daltonlens")
    package.__path__ = [str(clone / "daltonlens")]
    sys.modules["daltonlens"] = package
    import daltonlens.convert as convert
    import daltonlens.simulate as simulate
    return convert, simulate


def rows(m):
    return [[float(x) for x in r] for r in m]


def dump(value, indent=""):
    """JSON with each member of an object on its own line, and each array of numbers or rows, and each colour, on one."""
    if isinstance(value, dict):
        inner = indent + "  "
        members = [f"{inner}{json.dumps(k)}: {dump(v, inner)}" for k, v in value.items()]
        return "{\n" + ",\n".join(members) + "\n" + indent + "}"
    if isinstance(value, list) and value and isinstance(value[0], dict):
        inner = indent + "  "
        line = [inner + json.dumps(v, separators=(", ", ": ")) for v in value]
        return "[\n" + ",\n".join(line) + "\n" + indent + "]"
    return json.dumps(value, separators=(", ", ": "))


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: generate.py <clone of DaltonLens-Python>")
    clone = Path(sys.argv[1]).resolve()
    head = git(clone, "rev-parse", "HEAD")
    if head != COMMIT:
        sys.exit(f"the clone is at {head}, not the pinned {COMMIT}")
    if git(clone, "status", "--porcelain"):
        sys.exit("the clone has local changes")
    convert, simulate = load(clone)
    import numpy as np

    model = convert.LMSModel_sRGB_SmithPokorny75()
    vienot = simulate.Simulator_Vienot1999(model)
    brettel = simulate.Simulator_Brettel1997(model)
    assert brettel.use_white_as_neutral and not brettel.use_vischeck_anchors

    srgb8 = [(r, g, b) for r in LATTICE for g in LATTICE for b in LATTICE] + EXTRA
    linear = convert.linearRGB_from_sRGB(np.array(srgb8, dtype=np.float64) / 255.0)

    protan = vienot._simulate_dichromacy_linear_rgb(linear.copy(), simulate.Deficiency.PROTAN)
    vienot_protan = vienot.cvd_linear_rgb
    deutan = vienot._simulate_dichromacy_linear_rgb(linear.copy(), simulate.Deficiency.DEUTAN)
    vienot_deutan = vienot.cvd_linear_rgb
    # dumpPrecomputedValues keeps Brettel's matrices on the object (simulate.py:283-284, :314-321); what it prints is
    # discarded.
    brettel.dumpPrecomputedValues = True
    with contextlib.redirect_stdout(io.StringIO()):
        tritan = brettel._simulate_dichromacy_linear_rgb(linear.copy(), simulate.Deficiency.TRITAN)

    goldens = {
        "source": {
            "repository": REPOSITORY,
            "commit": COMMIT,
            "version": "0.1.6 (setup.cfg)",
            "ruling": "R-383",
            "generator": "fixtures/cvd/generate.py",
            "command": "python3 fixtures/cvd/generate.py <clone at the commit>",
            "numpy": np.__version__,
            "path": "_simulate_dichromacy_linear_rgb, linear sRGB in and out, severity 1",
        },
        "matrices": {
            "LMS_from_linearRGB": rows(model.LMS_from_linearRGB),
            "linearRGB_from_LMS": rows(model.linearRGB_from_LMS),
            "vienot_protan": rows(vienot_protan),
            "vienot_deutan": rows(vienot_deutan),
            "brettel_tritan": {
                "H1": rows(brettel.H1),
                "H2": rows(brettel.H2),
                "n_sep_lms": [float(x) for x in brettel.n_sep_plane],
                "T1": rows(brettel.T1),
                "T2": rows(brettel.T2),
                "n_sep_rgb": [float(x) for x in brettel.n_sep_plane_rgb],
            },
        },
        "colours": [
            {
                "srgb8": list(s),
                "linear": [float(x) for x in linear[k]],
                "protan": [float(x) for x in protan[k]],
                "deutan": [float(x) for x in deutan[k]],
                "tritan": [float(x) for x in tritan[k]],
            }
            for k, s in enumerate(srgb8)
        ],
    }
    OUT.write_text(dump(goldens) + "\n")
    print(f"wrote {len(srgb8)} colours to {OUT}")


if __name__ == "__main__":
    main()
