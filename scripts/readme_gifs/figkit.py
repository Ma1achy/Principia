"""Shared drawing kit for the Principia explainer GIFs.

Set PRINCIPIA_THEME=dark for the dark palette (default is light, ink on paper).
Set PRINCIPIA_OUT to choose where GIFs and previews are written (default: ../out).
"""
import io
import os
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from PIL import Image

THEME = os.environ.get("PRINCIPIA_THEME", "light").lower()
DARK = THEME == "dark"

if DARK:   # matches GitHub's dark canvas, so the GIF sits flush on the page
    BG, INK, GREY, LIGHT, ACC = "#0D1117", "#EDE8DA", "#8B8678", "#333945", "#E8664C"
else:      # ink on paper
    BG, INK, GREY, LIGHT, ACC = "#EFEBDD", "#15120F", "#8F8979", "#D6D0BF", "#C8442B"

from matplotlib import font_manager as _fm

_INSTALLED = {f.name for f in _fm.fontManager.ttflist}


def _pick(candidates):
    """First installed family from the list, so missing fonts never spam warnings."""
    return next((c for c in candidates if c in _INSTALLED), candidates[-1])


SANS = _pick(["Liberation Sans", "Arial", "Helvetica", "DejaVu Sans"])
MONO = _pick(["Liberation Mono", "Menlo", "Courier New", "DejaVu Sans Mono"])

OUT_DIR = os.environ.get("PRINCIPIA_OUT", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "out"))
PREV_DIR = os.path.join(OUT_DIR, "preview")


def gif_path(name):
    """Where a finished GIF goes: NAME.gif for light, NAME-dark.gif for dark."""
    os.makedirs(OUT_DIR, exist_ok=True)
    return os.path.join(OUT_DIR, f"{name}{'-dark' if DARK else ''}.gif")


def prev_path(name):
    os.makedirs(PREV_DIR, exist_ok=True)
    return os.path.join(PREV_DIR, f"{name}{'-dark' if DARK else ''}.png")


W, H, DPI = 9.6, 5.4, 100

plt.rcParams["font.family"] = SANS
plt.rcParams["axes.unicode_minus"] = False


def ease(x):
    x = float(np.clip(x, 0.0, 1.0))
    return x * x * (3 - 2 * x)


def new_fig(num, title, caption):
    fig = plt.figure(figsize=(W, H), dpi=DPI, facecolor=BG)
    fig.text(0.04, 0.925, f"FIG. {num}", family=MONO, size=9, color=GREY, va="center")
    fig.text(0.96, 0.925, "PRINCIPIA  ·  PLANAR THREE-BODY PROBLEM", family=MONO,
             size=8, color=GREY, va="center", ha="right")
    fig.text(0.04, 0.865, title, size=18, weight="bold", color=INK, va="center")
    fig.add_artist(Line2D([0.04, 0.96], [0.82, 0.82], color=INK, lw=0.9))
    fig.text(0.04, 0.045, caption, size=9.5, color=GREY, va="center")
    return fig


def style_axes(ax, spines=True):
    ax.set_facecolor(BG)
    for s in ["top", "right"]:
        ax.spines[s].set_visible(False)
    for s in ["left", "bottom"]:
        ax.spines[s].set_color(GREY)
        ax.spines[s].set_linewidth(0.8)
        ax.spines[s].set_visible(spines)
    ax.tick_params(colors=GREY, labelsize=7, length=3, width=0.7)
    for lab in ax.get_xticklabels() + ax.get_yticklabels():
        lab.set_family(MONO)


def bare_axes(ax):
    ax.set_facecolor(BG)
    ax.set_xticks([])
    ax.set_yticks([])
    for s in ax.spines.values():
        s.set_visible(False)


def grab(fig):
    fig.canvas.draw()
    buf = np.asarray(fig.canvas.buffer_rgba())
    img = Image.fromarray(buf[..., :3].copy())
    plt.close(fig)
    return img


def _palette_image(colors=32):
    """Fixed palette so frames never flicker: gradients between the ink colours."""
    def hexrgb(h):
        return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))
    seed = Image.new("RGB", (256, 6 * 8))
    px = seed.load()
    pairs = [(BG, INK), (BG, GREY), (BG, LIGHT), (BG, ACC), (LIGHT, INK), (GREY, INK)]
    for row, (a, b) in enumerate(pairs):
        ra, rb = hexrgb(a), hexrgb(b)
        for x in range(256):
            t = x / 255
            c = tuple(int(round(ra[i] + (rb[i] - ra[i]) * t)) for i in range(3))
            for y in range(8):
                px[x, row * 8 + y] = c
    return seed.quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)


def save_gif(frames, path, fps=20):
    pal = _palette_image()
    q = [f.quantize(palette=pal, dither=Image.Dither.NONE) for f in frames]
    q[0].save(path, save_all=True, append_images=q[1:], duration=int(1000 / fps),
              loop=0, optimize=False, disposal=1)
