"""Digitise the quantitative figures of Valtonen et al. 2016 and redraw them.

Usage:  python3 -I digitise.py <path/to/valtonen2016.pdf> <output dir (the valtonen2016/ folder)>

The book PDF is NOT in the repository (copyright). This script reads the embedded
raster images straight from the PDF (pymupdf), so its outputs are reproducible by
anyone holding the same PDF (SHA-256 251029ab...cf99cb, see ../README.md).

Every calibration below is measured on the image itself (frame lines and tick
marks), not assumed from the axis labels; the measured pixel positions are written
into the CSV headers so they can be checked.
"""
import sys, os, csv, io
import numpy as np
import pymupdf
from PIL import Image
from scipy import ndimage
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

PDF, OUT = sys.argv[1], sys.argv[2]
DATA = os.path.join(OUT, "data"); FIG = os.path.join(OUT, "figures")
os.makedirs(DATA, exist_ok=True); os.makedirs(FIG, exist_ok=True)
doc = pymupdf.open(PDF)

# categorical slots 1-3 and the blue sequential ramp of the house palette
C1, C2, C3 = "#2a78d6", "#eb6834", "#1baf7a"
INK, INK2 = "#0b0b0b", "#52514e"
BLUE_RAMP = ["#fcfcfb", "#cde2fb", "#9ec5f4", "#6da7ec", "#3987e5", "#256abf", "#184f95", "#0d366b"]
plt.rcParams.update({"font.size": 9, "axes.edgecolor": INK2, "axes.labelcolor": INK,
                     "xtick.color": INK2, "ytick.color": INK2, "axes.linewidth": 0.8})


def page_image(pdf_page):
    """Largest embedded image on a 1-based PDF page, as an RGB array."""
    pg = doc[pdf_page - 1]
    imgs = sorted(pg.get_images(full=True), key=lambda t: -t[2] * t[3])
    pix = pymupdf.Pixmap(doc, imgs[0][0])
    if pix.n - pix.alpha != 3:
        pix = pymupdf.Pixmap(pymupdf.csRGB, pix)
    return np.array(Image.open(io.BytesIO(pix.tobytes("png"))).convert("RGB")).astype(float)


def note(ax, fig_no):
    ax.text(0.0, -0.13, f"Redrawn from digitised data, after Valtonen et al. 2016, Fig. {fig_no}",
            transform=ax.transAxes, fontsize=7, color=INK2, va="top")


def write_csv(name, header_lines, cols, rows):
    with open(os.path.join(DATA, name), "w", newline="") as f:
        for h in header_lines:
            f.write("# " + h + "\n")
        w = csv.writer(f); w.writerow(cols)
        for r in rows:
            w.writerow(r)


# ----------------------------------------------------------------- Fig. 3.12
write_csv("fig3_12_pythagorean_ics.csv",
          ["Valtonen et al. 2016, Fig. 3.12, p. 75 (PDF p. 86). Values are the printed labels, not digitised.",
           "Credit in the caption: Szebehely & Peters 1967, AJ 72, 876 (doi:10.1086/110355), their Fig. 1.",
           "Bodies start at rest (book p. 74: 'initially at rest at the corners of a Pythagorean right triangle')."],
          ["body", "mass", "x", "y"],
          [[1, 3, 1, 3], [2, 4, -2, -1], [3, 5, 1, -1]])

# ----------------------------------------------------------------- Fig. 3.16
a = page_image(90)                      # 397 x 365 px, 150 ppi
R, G, B = a[..., 0], a[..., 1], a[..., 2]
# frame and ticks measured on the image: x: col 1 -> -3, col 395 -> 4 ; y: row 1 -> 3.5, row 363 -> -1.5
X = lambda c: -3 + (c - 1) / (394 / 7)
Y = lambda r: 3.5 - (r - 1) / (362 / 5)
marks = {}
for name, m in {"m3_initial (magenta point)": (R > 150) & (B > 130) & (G < 110),
                "m4_initial (cyan point)": (G > 170) & (B > 170) & (R < 170),
                "m5_initial (yellow point)": (R > 190) & (G > 190) & (B < 120)}.items():
    lab, n = ndimage.label(m); sz = ndimage.sum(m, lab, range(1, n + 1)); i = int(np.argmax(sz)) + 1
    r, c = ndimage.center_of_mass(m, lab, i); marks[name] = (X(c), Y(r))
# open circles (bounce positions): ring centres read off the pixel grid (see figures.md)
marks["m3_at_bounce (black open circle)"] = (X(273.5), Y(191.0))
marks["m4_m5_collision (brown open circle)"] = (X(135.0), Y(275.5))
rows = [[k, f"{v[0]:.3f}", f"{v[1]:.3f}"] for k, v in marks.items()]
write_csv("fig3_16_mikkola_markers.csv",
          ["Valtonen et al. 2016, Fig. 3.16, p. 79 (PDF p. 90). Caption credit: Seppo Mikkola. No published source is given.",
           "Calibration on the embedded raster (397x365 px): frame x col 1 = -3, col 395 = 4 (56.29 px/unit);",
           "y row 1 = 3.5, row 363 = -1.5 (72.40 px/unit); tick spacing checks to 0.5 px.",
           "Uncertainty: +-1 px on a marker centre = +-0.018 in x, +-0.014 in y; markers are ~10 px discs.",
           "Body identity: m-3 magenta point / red track, m-4 cyan point / green track, m-5 yellow point / blue track",
           "(each track starts at its point). Masses 3, 4, 5 (book p. 76)."],
          ["marker", "x", "y"], rows)
tracks = {"m3 (red track)": (R > 170) & (G < 120) & (B < 120) & ~((R > 150) & (B > 130)),
          "m4 (green track)": (G > 140) & (R < 160) & (B < 160) & (G - R > 30),
          "m5 (blue track)": (B > 110) & (R < 120) & (G < 120) & (B - R > 40)}
trows = []
for name, m in tracks.items():
    m[:3, :] = m[-3:, :] = False; m[:, :3] = m[:, -3:] = False
    rr, cc = np.nonzero(m)
    for r, c in zip(rr, cc):
        trows.append([name.split()[0], f"{X(c):.3f}", f"{Y(r):.3f}"])
write_csv("fig3_16_mikkola_tracks.csv",
          ["Valtonen et al. 2016, Fig. 3.16. One row per track pixel classified by colour; UNORDERED point cloud.",
           "Line width ~2 px: positional uncertainty ~+-0.02. Crossings and the markers themselves are excluded or mixed.",
           "Calibration as in fig3_16_mikkola_markers.csv."],
          ["body", "x", "y"], trows)
fig, ax = plt.subplots(figsize=(6, 4.6))
for (name, col) in zip(["m3", "m4", "m5"], [C2, C3, C1]):
    pts = np.array([[float(r[1]), float(r[2])] for r in trows if r[0] == name])
    ax.plot(pts[:, 0], pts[:, 1], ".", ms=1.2, color=col, label=f"{name} track (digitised pixels)")
for k, (x, y) in marks.items():
    filled = "initial" in k
    ax.plot(x, y, "o", ms=8, mfc=("none" if not filled else INK), mec=INK, mew=1.2)
    ax.annotate(k.split(" (")[0], (x, y), xytext=(6, 4), textcoords="offset points", fontsize=7, color=INK)
ax.set_xlim(-3, 4); ax.set_ylim(-1.5, 3.5); ax.set_aspect("equal"); ax.set_xlabel("x"); ax.set_ylabel("y")
ax.legend(fontsize=7, loc="upper left", frameon=False); ax.set_title("Near-Pythagorean bounce orbit (masses 3, 4, 5)", fontsize=9)
note(ax, "3.16"); fig.tight_layout(); fig.savefig(os.path.join(FIG, "replot_fig3_16_mikkola.png"), dpi=200); plt.close(fig)

# ----------------------------------------------------------------- Fig. 4.1
a = page_image(97).mean(axis=2)          # 576 x 412 px grey
X = lambda c: -1.5 + (c - 24) / 180.33   # frame cols 24 (-1.5) and 565 (1.5); top ticks every 90 px
Y = lambda r: (359 - r) / 176.5          # y ticks at rows 359, 271, 182.5, 94, 6 for 0 .. 2
rr, cc = np.nonzero(a < 235); x, y = X(cc), Y(rr)
keep = (x > 0.015) & (x < 1.05) & (y > 0.01) & (y < 1.8)
x, y = x[keep], y[keep]
arc = np.abs(np.hypot(x + 1, y) - 2) < 0.03
lab = np.zeros_like(x, bool)
for cx, cy, hx, hy in [(0.05, 1.55, .06, .09), (0.19, 1.31, .05, .05), (0.5, 1.0, .08, .1), (0.19, 0.70, .05, .05),
                       (0.07, 0.26, .05, .09), (0.19, 0.15, .05, .05), (0.75, 0.29, .05, .09), (0.69, 0.15, .05, .05)]:
    lab |= (np.abs(x - cx) < hx) & (np.abs(y - cy) < hy)
rest = ~arc & ~lab
x, y = x[rest], y[rest]; db = np.hypot(x - 1, y)
LM = y > 0.85
H = ~LM & (x > 0.44) & (np.abs(db - 0.53) < 0.06)
MA = ~LM & ~H & (x < 0.52) & (y > 0.12)
brows = []
for nm, m in [("L/M", LM), ("H boundary", H), ("M/A", MA)]:
    o = np.argsort(x[m])
    for xi, yi in zip(x[m][o], y[m][o]):
        brows.append([nm, f"{xi:.4f}", f"{yi:.4f}"])
write_csv("fig4_1_region_boundaries.csv",
          ["Valtonen et al. 2016, Fig. 4.1, p. 86 (PDF p. 97). No credit in the caption; layout as Heinamaki et al. 1999 Fig. 1.",
           "Coordinates as drawn: a = (-1,0), b = (1,0), c = (0,0), d = (0, sqrt 3); outer arc = circle radius 2 about a.",
           "Calibration on the raster (576x412 px): x = -1.5 + (col-24)/180.33, y = (359-row)/176.5 (frame and tick marks).",
           "Check: the digitised outer arc lies at 1.9982 +- 0.0034 from a (expected 2).",
           "Dashed/dotted boundaries: one row per dark pixel (threshold grey < 235), letters and asterisks masked out.",
           "Pixel = 0.0055 in x, 0.0057 in y; dashed lines are 1-2 px wide."],
          ["boundary", "x", "y"], brows)


def circ(x, y):
    A = np.c_[2 * x, 2 * y, np.ones_like(x)]; s, *_ = np.linalg.lstsq(A, x ** 2 + y ** 2, rcond=None)
    r = np.sqrt(s[2] + s[0] ** 2 + s[1] ** 2); return s[0], s[1], r, (np.hypot(x - s[0], y - s[1]) - r).std()


fits = []
for nm, m in [("L/M", LM), ("H boundary", H)]:
    d = np.hypot(x[m] - 1, y[m]); cx, cy, r, sd = circ(x[m], y[m])
    fits.append([nm, m.sum(), f"{x[m].min():.3f}", f"{x[m].max():.3f}", f"{d.mean():.4f}", f"{d.std():.4f}",
                 f"centre ({cx:.3f}, {cy:.3f}) r {r:.4f} rms {sd:.4f}", ""])
p = np.polyfit(x[MA], y[MA], 1); res = y[MA] - np.polyval(p, x[MA])
fits.append(["M/A", MA.sum(), f"{x[MA].min():.3f}", f"{x[MA].max():.3f}", "", "", "",
             f"y = {p[1]:.4f} {p[0]:+.4f} x, rms {res.std():.4f}"])
write_csv("fig4_1_boundary_fits.csv",
          ["Fits to fig4_1_region_boundaries.csv. These are OUR descriptions of the drawn curves; the book gives no definition.",
           "dist_b = distance of each boundary pixel from body b at (1,0)."],
          ["boundary", "n_px", "x_min", "x_max", "dist_b_mean", "dist_b_sd", "free_circle_fit", "line_fit"], fits)
pts = {"1 (M)": (0.4979, 0.9632), "2 (L)": (0.0490, 1.5020), "3 (A)": (0.0749, 0.2193), "4 (H)": (0.7487, 0.2493)}
write_csv("fig4_1_example_points.csv",
          ["Valtonen et al. 2016, Fig. 4.1: centroids of the four asterisks (dark pixels < 150 within +-6 px). +-0.006."],
          ["point", "x", "y"], [[k, v[0], v[1]] for k, v in pts.items()])
fig, ax = plt.subplots(figsize=(5.2, 4.6))
t = np.linspace(0, np.pi / 3, 200)
ax.plot(-1 + 2 * np.cos(t), 2 * np.sin(t), color=INK2, lw=1)
ax.plot([0, 0], [0, np.sqrt(3)], color=INK2, lw=1, ls=":"); ax.plot([-1, 1], [0, 0], color=INK2, lw=1)
ax.plot([-1, 0], [0, np.sqrt(3)], color=INK2, lw=1, ls="--")
for (nm, m), col in zip([("L/M", LM), ("H boundary", H), ("M/A", MA)], [C1, C2, C3]):
    ax.plot(x[m], y[m], ".", ms=2.5, color=col, label=f"{nm} (digitised)")
for k, (px, py) in pts.items():
    ax.plot(px, py, "x", color=INK, ms=6); ax.annotate(k, (px, py), xytext=(5, 3), textcoords="offset points", fontsize=7)
for lbl, (px, py) in {"a": (-1, 0), "b": (1, 0), "c": (0, 0), "d": (0, np.sqrt(3))}.items():
    ax.annotate(lbl, (px, py), xytext=(-9, -10), textcoords="offset points", fontsize=8, color=INK)
ax.set_xlim(-1.1, 1.1); ax.set_ylim(-0.1, 1.85); ax.set_aspect("equal"); ax.legend(fontsize=7, frameon=False, loc="upper left")
ax.set_title("Region D and its L/M/A/H partition", fontsize=9); note(ax, "4.1")
fig.tight_layout(); fig.savefig(os.path.join(FIG, "replot_fig4_1_region_partition.png"), dpi=200); plt.close(fig)

# ----------------------------------------------------------------- Figs. 4.5 and 4.9: colour-class rasters
def raster(a, x, y, region, signed, step, fname, hdr, fig_no, title, extent, label):
    rows = []
    xs = np.arange(extent[0], extent[1] - 1e-9, step); ys = np.arange(extent[2], extent[3] - 1e-9, step)
    grid = np.full((len(ys), len(xs)), np.nan)
    for j, y0 in enumerate(ys):
        for i, x0 in enumerate(xs):
            m = region & (x >= x0) & (x < x0 + step) & (y >= y0) & (y < y0 + step)
            if m.sum() < 4:
                continue
            frac = (signed[m] > 25).mean(); grid[j, i] = frac
            rows.append([f"{x0 + step / 2:.3f}", f"{y0 + step / 2:.3f}", int(m.sum()), f"{frac:.3f}"])
    write_csv(fname, hdr, ["x_centre", "y_centre", "n_pixels", label], rows)
    fig, ax = plt.subplots(figsize=(4.2 if extent[1] - extent[0] < 1.5 else 3.6, 5))
    from matplotlib.colors import LinearSegmentedColormap
    cmap = LinearSegmentedColormap.from_list("blue", BLUE_RAMP)
    im = ax.imshow(grid, origin="lower", extent=extent, cmap=cmap, vmin=0, vmax=1, interpolation="nearest")
    ax.set_aspect("equal"); cb = fig.colorbar(im, ax=ax, shrink=0.7); cb.set_label(label.replace("_", " "), fontsize=7)
    ax.set_title(title, fontsize=9); note(ax, fig_no); fig.tight_layout()
    fig.savefig(os.path.join(FIG, f"replot_fig{fig_no.replace('.', '_')}_class_fraction.png"), dpi=200); plt.close(fig)


a = page_image(105); H_, W_, _ = a.shape                 # 461 x 747 px
cc, rr = np.meshgrid(np.arange(W_), np.arange(H_))
s45, c0, r0 = 201.0, 248.0, 364.5        # lens extremes: cols 47..449 (x=-1..1), rows 16..713 (y=+-sqrt 3)
x = (cc - c0) / s45; y = (r0 - rr) / s45
lens = (np.hypot(x + 1, y) < 1.98) & (np.hypot(x - 1, y) < 1.98)
raster(a, x, y, lens, a[..., 2] - a[..., 0], 0.04, "fig4_5_blue_fraction_raster.csv",
       ["Valtonen et al. 2016, Fig. 4.5, p. 94 (PDF p. 105). Caption: blue = short-lived, yellow = long-lived; NO colour scale is given.",
        "So this is a colour CLASS, not a lifetime: blue_fraction = share of pixels in the cell with B - R > 25 (8-bit).",
        "Calibration from the lens itself (raster 461x747 px): x = (col-248)/201, y = (364.5-row)/201; a 1-px shift is 0.005.",
        "The class fraction depends strongly on the threshold (lens total: 0.71 at B-R>20, 0.43 at >40, 0.23 at >60).",
        "Cells 0.04 x 0.04; cells with < 4 lens pixels are omitted."],
       "4.5", "Fig. 4.5 as a blue-class fraction", [-1, 1, -1.76, 1.76], "blue_fraction")
a = page_image(108); H_, W_, _ = a.shape                 # 461 x 775 px
cc, rr = np.meshgrid(np.arange(W_), np.arange(H_))
x = (cc - 32) / 420.0; y = (743 - rr) / 420.0
reg = (x > 0.01) & (y > 0.01) & (np.hypot(x + 1, y) < 1.99)
raster(a, x, y, reg, a[..., 0] - a[..., 2], 0.02, "fig4_9_brown_fraction_raster.csv",
       ["Valtonen et al. 2016, Fig. 4.9, p. 97 (PDF p. 108). Caption: low entropy blue, high entropy light brown; NO scale given.",
        "brown_fraction = share of pixels in the cell with R - B > 25 (8-bit). Calibration (461x775 px): x = (col-32)/420,",
        "y = (743-row)/420, from the region's own extremes (x 0..1 at cols 32..452, y 0..sqrt3 at rows 743..16).",
        "Whole-region totals by threshold on R-B: >0: 0.827 brown / 0.173 blue; >25: 0.779 / 0.144; >60: 0.656 / 0.115.",
        "Cells 0.02 x 0.02; cells with < 4 region pixels are omitted."],
       "4.9", "Fig. 4.9 as a brown-class fraction", [0, 1, 0, 1.74], "brown_fraction")

# ----------------------------------------------------------------- Fig. 8.1
P = {("m2", 1): (171.75, 9.0), ("m2", 2): (123.0, 93.25), ("m2", 3): (127.0, 175.0), ("m2", 4): (151.75, 233.75),
     ("m2", 4.56): (181.0, 282.5), ("m1", 1): (224.25, 86.0), ("m1", 2): (215.75, 79.0), ("m1", 3): (156.0, 88.25),
     ("m1", 4): (77.0, 159.0), ("m1", 4.56): (49.0, 224.5), ("m3", 1): (96.5, 489.5), ("m3", 2): (125.75, 449.25),
     ("m3", 3): (153.75, 405.0), ("m3", 4): (180.75, 341.25), ("m3", 4.56): (177.0, 286.25)}
mass = {"m1": 0.5, "m2": 0.5, "m3": 1.0}
com = np.mean([[sum(mass[b] * P[(b, t)][k] for b in mass) / 2 for k in (0, 1)] for t in (1, 2, 3, 4)], axis=0)
unit = 102.0
rows = [[b, mass[b], t, f"{(P[(b, t)][0] - com[0]) / unit:.3f}", f"{-(P[(b, t)][1] - com[1]) / unit:.3f}",
         f"{P[(b, t)][0]:.2f}", f"{P[(b, t)][1]:.2f}"] for (b, t) in P]
write_csv("fig8_1_valtonen1976_positions.csv",
          ["Valtonen et al. 2016, Fig. 8.1, p. 148 (PDF p. 158). Credit: Valtonen 1976, A&A 46, 435 (no DOI on Crossref).",
           "Open-circle centres at the labelled times, refined by a ring-score search on the 301x504 px raster; +-0.5 px.",
           "Scale: the '1 UNIT' bar is 102 px (cols 25-127 on row 322), +-1.5 px, i.e. +-1.5 per cent on every distance.",
           "Origin: the mass-weighted centre at t = 1, 2, 3, 4, which is constant to 1 px (pixel COM per time in px_x/px_y);",
           "at t = 4.56 it is 1.6 px off. y is up. Time unit as in the figure (not defined in the book).",
           "Masses from the labels: m1 = 0.5, m2 = 0.5, m3 = 1.0. At t = 4.56 m2 and m3 coincide (collision); m1 leaves."],
          ["body", "mass", "t", "x_units", "y_units", "px_x", "px_y"], rows)
fig, ax = plt.subplots(figsize=(4, 5.4))
for (b, col) in zip(["m1", "m2", "m3"], [C1, C2, C3]):
    ts = [1, 2, 3, 4, 4.56]; xy = np.array([[(P[(b, t)][0] - com[0]) / unit, -(P[(b, t)][1] - com[1]) / unit] for t in ts])
    ax.plot(xy[:, 0], xy[:, 1], "-o", color=col, ms=5, lw=1.2, label=f"{b} (mass {mass[b]})")
    for t, (px, py) in zip(ts, xy):
        ax.annotate(f"{t:g}", (px, py), xytext=(4, 2), textcoords="offset points", fontsize=7, color=INK)
ax.plot(0, 0, "+", color=INK2); ax.set_aspect("equal"); ax.legend(fontsize=7, frameon=False, loc="lower left")
ax.set_xlabel("x (figure units)"); ax.set_ylabel("y"); ax.set_title("Valtonen (1976) relativistic triple: digitised markers\n(straight segments join markers; not the orbit)", fontsize=8)
note(ax, "8.1"); fig.tight_layout(); fig.savefig(os.path.join(FIG, "replot_fig8_1_markers.png"), dpi=200); plt.close(fig)

# ----------------------------------------------------------------- axis windows (read off the tick labels)
write_csv("fig4_5to4_9_axis_windows.csv",
          ["Axis ranges of the map panels: zoom windows (4.6 right, 4.7) are the printed end tick labels; the full-shield and quarter extents (+-sqrt 3) are the region geometry, drawn to them (book coordinates, bodies at (-1,0) and (1,0)).",
           "Lehto et al. 2008 put the bodies at (0,0) and (1,0): x_L = (x+1)/2, y_L = y/2 (their lens is half the size)."],
          ["figure", "page", "panel", "x_min", "x_max", "y_min", "y_max", "x_L_min", "x_L_max", "y_L_min", "y_L_max"],
          [["4.5", 94, "full shield", -1.0, 1.0, -1.732, 1.732, 0.0, 1.0, -0.866, 0.866],
           ["4.6", 95, "left (quarter)", 0.0, 1.0, 0.0, 1.732, 0.5, 1.0, 0.0, 0.866],
           ["4.6", 95, "right", 0.40, 0.60, 0.90, 1.10, 0.70, 0.80, 0.45, 0.55],
           ["4.7", 95, "left", 0.500, 0.520, 0.910, 0.930, 0.750, 0.760, 0.455, 0.465],
           ["4.7", 95, "right", 0.5080, 0.5100, 0.9140, 0.9160, 0.7540, 0.7550, 0.4570, 0.4580],
           ["4.9", 97, "quarter", 0.0, 1.0, 0.0, 1.732, 0.5, 1.0, 0.0, 0.866]])
print("done")
