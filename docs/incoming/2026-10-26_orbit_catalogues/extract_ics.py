"""Extract initial conditions and metadata from a Three Body Orbits snapshot into one CSV.

Usage: python3 -I extract_ics.py <catalogue.json> <orbits_dir> <out.csv>

<orbits_dir> holds https://data.threebodyorbits.com/orbits/<key>.bin for every key in the catalogue.
The file layout is the site's own (js/orbit-data.js, read 26 Oct 2026), little endian:
  0 magic "TBO2"/"TBO3"; 4 u32 version; 8 f64 period; 16 f64 masses[3]; 40 f64 bbox[4];
  72 f64 initial conditions as published[12]; 168 f64 refined initial conditions[12]; ...
The order of the 12 values is x1 y1 x2 y2 x3 y3 vx1 vy1 vx2 vy2 vx3 vy3. That order is inferred, not documented: it is
confirmed by recomputing E and L (G = 1) from the refined initial conditions and comparing them with the catalogue's
own E and L (relative E agreement better than 5e-7 and L better than 5e-6 on all 3,943 rows, the catalogue's printed
precision).
"""

import csv
import json
import math
import struct
import sys

NAMES = "x1 y1 x2 y2 x3 y3 vx1 vy1 vx2 vy2 vx3 vy3".split()


def energy_and_lz(m, s):
    r = [(s[0], s[1]), (s[2], s[3]), (s[4], s[5])]
    v = [(s[6], s[7]), (s[8], s[9]), (s[10], s[11])]
    kinetic = sum(0.5 * m[i] * (v[i][0] ** 2 + v[i][1] ** 2) for i in range(3))
    potential = -sum(
        m[i] * m[j] / math.hypot(r[i][0] - r[j][0], r[i][1] - r[j][1]) for i in range(3) for j in range(i + 1, 3)
    )
    lz = sum(m[i] * (r[i][0] * v[i][1] - r[i][1] * v[i][0]) for i in range(3))
    return kinetic + potential, lz


def main(catalogue_path, orbits_dir, out_path):
    orbits = json.load(open(catalogue_path))["orbits"]
    rows = []
    for o in orbits:
        b = open(f"{orbits_dir}/{o['key']}.bin", "rb").read()
        if b[:4] not in (b"TBO2", b"TBO3"):
            raise SystemExit(f"{o['key']}: not an orbit file")
        period = struct.unpack_from("<d", b, 8)[0]
        m = struct.unpack_from("<3d", b, 16)
        pub = struct.unpack_from("<12d", b, 72)
        ref = struct.unpack_from("<12d", b, 168)
        e, lz = energy_and_lz(m, ref)
        row = {
            "key": o["key"], "family": o["family"], "code": o["code"], "source": o.get("source", ""),
            "m1": repr(m[0]), "m2": repr(m[1]), "m3": repr(m[2]), "T": repr(period),
            "E_cat": o["E"], "L_cat": o["L"], "E_recomp": repr(e), "L_recomp": repr(lz),
            "rmin": o["rmin"], "stab": o["stab"], "stable_pub": o.get("stable_pub", ""), "syz": o["syz"],
            "symmetry": o.get("symmetry", ""), "closure_published": o["closure0"], "closure_refined": o["closure"],
        }
        row.update({f"pub_{n}": repr(x) for n, x in zip(NAMES, pub)})
        row.update({f"ref_{n}": repr(x) for n, x in zip(NAMES, ref)})
        rows.append(row)
    with open(out_path, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)


if __name__ == "__main__":
    main(*sys.argv[1:4])
