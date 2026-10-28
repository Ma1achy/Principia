"""Read Fig. 1 of Boekholt, Portegies Zwart & Valtonen (2020) from the PDF's vector paths, reconcile each panel to
the paper's 1212 grid points, and write one CSV per panel. The paper's figure, read by us: no number here is computed
by us beyond the reading itself.

Usage (from this folder), in a venv with pymupdf and matplotlib:
  python3 -I build.py [PDF] | tee build_output.txt
PDF defaults to the arXiv copy in the repo:
  ../../../2026-10-25_replication/papers/boekholt_pz_valtonen_2020_MNRAS_493_3932_arXiv2002.04029.pdf
The script also takes the publisher copy (staa452.pdf); the figure is on page index 2 in both.

Source: T. C. N. Boekholt, S. F. Portegies Zwart & M. Valtonen, "Gargantuan chaotic gravitational three-body systems
and their irreversibility to the Planck length", MNRAS 493(3), 3932-3937 (2020), arXiv:2002.04029v1, Fig. 1
(left: eps = 1e-6; right: eps = 1e-70; colour log10 T on [0, 3]; black: T > 1000). The grid: "a uniform grid on the
map with a grid-spacing of 0.015625" giving "1212 initial conditions" (Sec. 2; Appendix A).

Method:
  1. Every filled path made only of Bezier segments on the page is a dot. Each panel's axes frame is the white filled
     rectangle that holds its dots; a dot belongs to the frame holding its centre.
  2. The axis ticks are the short stroked lines on the frame's bottom (x) and left (y) edges; each is paired with the
     numeric label printed beside it. A least-squares line through (tick position, label) maps page points to (x, y).
  3. (x, y) * 64 is snapped to the nearest integer (i, j). The script stops if any centre is more than 0.05 grid
     steps from its grid point.
  4. A fill is matched to matplotlib's 256-entry `jet` table by L1 distance. Pure black (0, 0, 0) is not a jet entry
     and means T > 1000: censored, not a number. jet entries 29, 30 and 31 are identical, so a dot of that colour
     reads as the LUT range 29-31.
  5. Under Normalize(0, 3) matplotlib puts log10 T in LUT bin k = floor(256 * log10 T / 3), clipped to 0..255, so bin
     k is log10 T in [3k/256, 3(k+1)/256). The CSV gives the bin's bounds and its midpoint.
  6. Reconciliation: each panel must hold exactly 1212 distinct grid points, each with exactly one coloured dot, and
     every further dot must be a black dot drawn over a coloured one at the same grid point. Anything else stops the
     script before any file is written.
"""
import collections
import csv
import hashlib
import sys

import matplotlib
import numpy as np
import pymupdf
from matplotlib import colormaps

DEFAULT_PDF = "../../../2026-10-25_replication/papers/boekholt_pz_valtonen_2020_MNRAS_493_3932_arXiv2002.04029.pdf"
PAGE = 2
N_GRID = 1212
STEP = 64  # grid points per unit: spacing 0.015625 = 1/64
SNAP_TOL = 0.05  # grid steps
COLOUR_TOL = 1e-4  # L1 distance to the nearest jet entry
PANELS = (("left", "1e-6"), ("right", "1e-70"))


def stop(msg):
    sys.exit(f"STOP: {msg}")


def centre(r):
    return ((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2)


def labels(page):
    out = []
    for b in page.get_text("dict")["blocks"]:
        for line in b.get("lines", []):
            for s in line["spans"]:
                try:
                    v = float(s["text"].strip())
                except ValueError:
                    continue
                out.append((v, pymupdf.Rect(s["bbox"])))
    return out


def axis_map(frame, ticks_x, ticks_y, labs):
    """Pair each tick with its label and fit page coordinate -> data coordinate per axis."""
    fits = {}
    for name, ticks, near in (
        ("x", ticks_x, lambda t, r: abs(centre(r)[0] - t) < 3 and 0 <= r.y0 - frame.y1 < 6),
        ("y", ticks_y, lambda t, r: abs(centre(r)[1] - t) < 3 and 0 <= frame.x0 - r.x1 < 6),
    ):
        pairs = []
        for t in sorted(set(round(v, 3) for v in ticks)):
            hit = [v for v, r in labs if near(t, r)]
            if len(hit) != 1:
                stop(f"{name} tick at {t} pt has {len(hit)} labels")
            pairs.append((t, hit[0]))
        p, v = np.array(pairs).T
        a, b = np.polyfit(p, v, 1)
        resid = np.abs(a * p + b - v).max()
        if resid > 1e-4:
            stop(f"{name} ticks not linear: residual {resid}")
        fits[name] = (a, b, pairs, resid)
    return fits


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PDF
    sha = hashlib.sha256(open(path, "rb").read()).hexdigest()
    print(f"source {path}\n  sha256 {sha}\n  page index {PAGE}")
    print(f"  pymupdf {pymupdf.__version__}, matplotlib {matplotlib.__version__}, numpy {np.__version__}")
    page = pymupdf.open(path)[PAGE]
    drawings = page.get_drawings()

    dots = [d for d in drawings if d.get("fill") is not None and d["items"] and all(it[0] == "c" for it in d["items"])]
    sizes = collections.Counter((round(d["rect"].width, 3), round(d["rect"].height, 3), len(d["items"])) for d in dots)
    print(f"\n== dots ==\n  {len(dots)} filled Bezier-only paths; (width pt, height pt, segments): {dict(sizes)}")

    rects = [d for d in drawings if d.get("fill") == (1.0, 1.0, 1.0) and len(d["items"]) == 1
             and d["items"][0][0] == "re"]
    frames = []
    for d in rects:
        r = d["rect"]
        inside = [k for k in dots if r.contains(pymupdf.Point(*centre(k["rect"])))]
        if inside and not any(o["rect"] != r and r.contains(o["rect"]) and o in rects for o in rects):
            frames.append(r)
    frames.sort(key=lambda r: r.x0)
    if len(frames) != 2:
        stop(f"found {len(frames)} axes frames, expected 2")

    segs = [d for d in drawings if len(d["items"]) == 1 and d["items"][0][0] == "l" and d.get("color") is not None]
    labs = labels(page)
    lut = colormaps["jet"](np.arange(256))[:, :3]
    ties = [(a, b) for a in range(256) for b in range(a + 1, 256) if np.abs(lut[a] - lut[b]).sum() == 0]
    print(f"  jet entries that are identical: {ties}")

    out = {}
    assigned = 0
    for (side, eps), fr in zip(PANELS, frames):
        tx = [d["items"][0][1].x for d in segs if abs(d["rect"].width) < 1e-6 and d["rect"].height < 3
              and abs(d["rect"].y1 - fr.y1) < 1e-3 and fr.x0 <= d["rect"].x0 <= fr.x1]
        ty = [d["items"][0][1].y for d in segs if abs(d["rect"].height) < 1e-6 and d["rect"].width < 3
              and abs(d["rect"].x0 - fr.x0) < 1e-3]
        fits = axis_map(fr, tx, ty, labs)
        (ax, bx, px, rx), (ay, by, py, ry) = fits["x"], fits["y"]
        print(f"\n== panel {side} (eps = {eps}) ==\n  frame {tuple(round(v, 3) for v in fr)}")
        print(f"  x ticks (pt, label) {[(round(t, 3), v) for t, v in px]}; fit residual {rx:.1e}")
        print(f"  y ticks (pt, label) {[(round(t, 3), v) for t, v in py]}; fit residual {ry:.1e}")
        print(f"  grid step in pt: x {1 / STEP / ax:.4f}, y {abs(1 / STEP / ay):.4f}")

        mine = [(n, d) for n, d in enumerate(dots) if fr.contains(pymupdf.Point(*centre(d["rect"])))]
        assigned += len(mine)
        cells = collections.defaultdict(list)
        worst = 0.0
        for n, d in mine:
            cx, cy = centre(d["rect"])
            fi, fj = (ax * cx + bx) * STEP, (ay * cy + by) * STEP
            i, j = round(fi), round(fj)
            worst = max(worst, abs(fi - i), abs(fj - j))
            fill = np.array(d["fill"])
            black = not fill.any()
            dist = np.abs(lut - fill).sum(1)
            best = np.flatnonzero(dist == dist.min())
            resid = float(fill.sum()) if black else float(dist.min())  # black: distance to (0, 0, 0)
            cells[(i, j)].append(dict(order=n, black=black, lut=best, resid=resid))
        if worst > SNAP_TOL:
            stop(f"{side}: a centre lies {worst:.3f} grid steps off the grid")
        n_black = sum(c["black"] for v in cells.values() for c in v)
        print(f"  paths {len(mine)}; black {n_black}; distinct grid points {len(cells)}; "
              f"largest snap offset {worst:.5f} grid steps")

        # Reconcile.
        pattern = collections.Counter()
        for (i, j), v in cells.items():
            colour = [c for c in v if not c["black"]]
            blk = [c for c in v if c["black"]]
            if len(colour) != 1 or len(blk) > 1:
                stop(f"{side} ({i},{j}): {len(colour)} coloured and {len(blk)} black dots")
            if blk and blk[0]["order"] < colour[0]["order"]:
                stop(f"{side} ({i},{j}): black dot drawn before the coloured one")
            if colour[0]["resid"] > COLOUR_TOL:
                stop(f"{side} ({i},{j}): fill is {colour[0]['resid']:.2e} from the nearest jet entry")
            pattern["coloured, then black over it" if blk else "coloured only"] += 1
        if len(cells) != N_GRID:
            stop(f"{side}: {len(cells)} grid points, not {N_GRID}")
        if len(mine) != N_GRID + n_black:
            stop(f"{side}: {len(mine)} paths != {N_GRID} + {n_black} black")
        under = collections.Counter(int(c["lut"][0]) for v in cells.values() if len(v) == 2
                                    for c in v if not c["black"])
        print(f"  per grid point: {dict(pattern)}")
        print(f"  RECONCILED: {len(mine)} paths = {N_GRID} coloured (one per grid point) + {n_black} black drawn over "
              f"a coloured dot at the same point; extra positions: none")
        print(f"  LUT index of the coloured dot under each black dot: {dict(under)}")
        print(f"  largest colour residual (L1, RGB on 0-1): {max(c['resid'] for v in cells.values() for c in v):.1e}")
        out[side] = (eps, cells)

    if assigned != len(dots):
        stop(f"{len(dots) - assigned} dots fall in neither frame")
    keys = [set(out[s][1]) for s, _ in PANELS]
    region = {(i, j) for i in range(-1, STEP + 1) for j in range(-1, STEP + 1)
              if 0 < i / STEP < 0.5 and j > 0 and (i / STEP + 0.5) ** 2 + (j / STEP) ** 2 < 1}
    print("\n== grid ==")
    print(f"  both panels hold the same {N_GRID} grid points: {keys[0] == keys[1]}")
    print(f"  equal to the 1/64 grid points with 0 < x < 1/2, y > 0, (x + 1/2)^2 + y^2 < 1 "
          f"({len(region)} points): {keys[0] == region}")
    print(f"  i range {min(i for i, _ in keys[0])}..{max(i for i, _ in keys[0])}, "
          f"j range {min(j for _, j in keys[0])}..{max(j for _, j in keys[0])}")

    for side, _ in PANELS:
        eps, cells = out[side]
        write(side, eps, cells, sha)


def write(side, eps, cells, sha):
    name = f"gg1_fig1_{side}_eps{eps}.csv"
    with open(name, "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["grid_index", "i", "j", "x", "y", "lut_index", "log10_T", "log10_T_lo", "log10_T_hi",
                    "colour_residual_L1", "draw_order", "covered_lut_index"])
        for n, (i, j) in enumerate(sorted(cells), start=1):
            v = cells[(i, j)]
            c = [c for c in v if not c["black"]][0]
            blk = [c for c in v if c["black"]]
            lo, hi = int(c["lut"].min()), int(c["lut"].max())
            idx = str(lo) if lo == hi else f"{lo}-{hi}"
            if blk:
                row = ["black", "censored >1000", "3", "", f"{blk[0]['resid']:.1e}", blk[0]["order"], idx]
            else:
                a, b = 3 * lo / 256, 3 * (hi + 1) / 256
                row = [idx, f"{(a + b) / 2:.9f}", f"{a:.9f}", f"{b:.9f}", f"{c['resid']:.1e}", c["order"], ""]
            w.writerow([n, i, j, f"{i / STEP:.6f}", f"{j / STEP:.6f}"] + row)
    print(f"  wrote {name}")


if __name__ == "__main__":
    main()
