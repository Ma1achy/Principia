"""FIG. 3 — scale and time: Burrau's 3-4-5 problem as Sun/Earth/Moon, as everyday balls, with no units (v4)."""
import sys
import numpy as np
from scipy.integrate import solve_ivp
from matplotlib.patches import Circle, Polygon, Ellipse, Arc, RegularPolygon
from figkit import *

# ---------- the problem itself (G = 1, masses 3, 4, 5, released from rest) ----------
MASS = np.array([3.0, 4.0, 5.0])
X0 = np.array([[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]])
T_END = 15.0


def _rhs(t, y):
    x, a = y[:6].reshape(3, 2), np.zeros((3, 2))
    for i in range(3):
        for j in range(3):
            if i != j:
                d = x[j] - x[i]
                a[i] += MASS[j] * d / np.linalg.norm(d) ** 3
    return np.concatenate([y[6:], a.ravel()])


_SOL = solve_ivp(_rhs, [0, T_END + 0.2], np.concatenate([X0.ravel(), np.zeros(6)]),
                 method="DOP853", rtol=1e-12, atol=1e-12, dense_output=True)


def pos(t):
    return _SOL.sol(min(max(t, 0.0), T_END))[:6].reshape(3, 2)


# ---------- two physical realisations ----------
G = 6.674e-11
M_SUN, AU = 1.989e30, 1.496e11
TAU_STAR = np.sqrt(AU ** 3 / (G * M_SUN))      # s, for 1 solar mass and 1 AU
TAU_BALL = np.sqrt(0.1 ** 3 / (G * 0.1))        # s, for 100 g and 10 cm
HOUR, DAY, YEAR = 3600.0, 86400.0, 365.25 * 86400.0


def fmt_t(sec):
    if sec < 100 * HOUR:
        return f"{sec / HOUR:5.1f} h"
    if sec < 700 * DAY:
        return f"{sec / DAY:5.1f} days"
    return f"{sec / YEAR:5.2f} yr"


def fmt_tau(sec):
    return f"{sec / HOUR:4.1f} hours" if sec < 100 * HOUR else f"{sec / DAY:4.1f} days"
RATIO = TAU_STAR / TAU_BALL

CX, CY, HALF = 0.1, 0.75, 3.2
RAYS = np.linspace(0, 2 * np.pi, 12, endpoint=False)


def art(ax, kind, i, x, y, r, alpha):
    """Draw body i (0, 1, 2 = mass 3, 4, 5) as a skin; plain dots for the unit-free panel."""
    if kind == "plain":
        ax.add_patch(Circle((x, y), 0.075 * np.sqrt(MASS[i]), fc=INK, ec=INK, alpha=alpha, zorder=6))
        return
    if kind == "cosmic":
        if i == 2:      # Sun
            ax.add_patch(Circle((x, y), r, fc=ACC, ec=ACC, alpha=alpha, zorder=6))
            for a in RAYS:
                ax.plot([x + 1.3 * r * np.cos(a), x + 1.85 * r * np.cos(a)],
                        [y + 1.3 * r * np.sin(a), y + 1.85 * r * np.sin(a)], color=ACC, lw=1.1,
                        alpha=alpha, zorder=6)
        elif i == 1:    # Earth
            ax.add_patch(Circle((x, y), r, fc=BG, ec=INK, lw=1.5, alpha=alpha, zorder=6))
            ax.add_patch(Ellipse((x, y), 1.0 * r, 2 * r, fill=False, ec=INK, lw=0.8, alpha=alpha, zorder=7))
            ax.plot([x - r, x + r], [y, y], color=INK, lw=0.8, alpha=alpha, zorder=7)
        else:           # Moon
            ax.add_patch(Circle((x, y), r, fc=LIGHT, ec=INK, lw=1.2, alpha=alpha, zorder=6))
            for dx, dy, rr in ((-0.35, 0.3, 0.2), (0.3, -0.1, 0.16), (-0.1, -0.45, 0.13)):
                ax.add_patch(Circle((x + dx * r, y + dy * r), rr * r, fc=GREY, ec=GREY, alpha=0.6 * alpha, zorder=7))
        return
    # everyday balls
    if i == 0:          # ping-pong ball
        ax.add_patch(Circle((x, y), r, fc=BG, ec=INK, lw=1.5, alpha=alpha, zorder=6))
        ang = np.radians(np.linspace(105, 175, 12))
        ax.plot(x + 0.62 * r * np.cos(ang), y + 0.62 * r * np.sin(ang), color=GREY, lw=1.2, alpha=alpha, zorder=7)
    elif i == 1:        # tennis ball
        ax.add_patch(Circle((x, y), r, fc=LIGHT, ec=INK, lw=1.5, alpha=alpha, zorder=6))
        ax.add_patch(Arc((x - 1.05 * r, y), 2.2 * r, 2.2 * r, theta1=-48, theta2=48, ec=INK, lw=1.2,
                         alpha=alpha, zorder=7))
        ax.add_patch(Arc((x + 1.05 * r, y), 2.2 * r, 2.2 * r, theta1=132, theta2=228, ec=INK, lw=1.2,
                         alpha=alpha, zorder=7))
    else:               # football
        ax.add_patch(Circle((x, y), r, fc=BG, ec=INK, lw=1.5, alpha=alpha, zorder=6))
        ax.add_patch(RegularPolygon((x, y), 5, radius=0.42 * r, orientation=np.pi, fc=INK, ec=INK,
                                    alpha=alpha, zorder=7))
        for k in range(5):
            a = np.pi / 2 + 2 * np.pi * k / 5
            ax.plot([x + 0.42 * r * np.cos(a), x + 0.98 * r * np.cos(a)],
                    [y + 0.42 * r * np.sin(a), y + 0.98 * r * np.sin(a)], color=INK, lw=1.0,
                    alpha=alpha, zorder=7)


def panel(ax, kind, t, alpha=1.0):
    bare_axes(ax)
    ax.set_xlim(CX - HALF, CX + HALF)
    ax.set_ylim(CY - HALF, CY + HALF)
    ax.set_aspect("equal", adjustable="box")
    col = ACC if kind == "cosmic" else INK
    ts = np.linspace(0, t, 1 + int(160 * t / T_END)) if t > 0 else np.array([0.0])
    path = np.array([pos(x) for x in ts])
    for i in range(3):
        ax.plot(path[:, i, 0], path[:, i, 1], color=LIGHT, lw=1.0, zorder=1, alpha=alpha)
        tail = path[max(0, len(ts) - 28):, i]
        ax.plot(tail[:, 0], tail[:, 1], color=col, lw=1.6, zorder=3, alpha=0.85 * alpha)
    P = pos(t)
    ax.add_patch(Polygon(P, closed=True, fill=False, ec=col, lw=1.0, alpha=0.9 * alpha, zorder=4))
    for i in range(3):
        m = MASS[i]
        x, y = P[i]
        r = 0.11 * np.sqrt(m) * 1.9
        art(ax, kind, i, x, y, r, alpha)
        ax.text(x - r - 0.12, y + r + 0.05, f"{int(m)}", family=MONO, size=9, color=INK, ha="right",
                va="bottom", alpha=alpha, zorder=8)
    return ax


def ruler(ax, label, alpha=1.0):
    x0, y0 = CX - HALF + 0.25, CY - HALF + 0.35
    ax.plot([x0, x0 + 1.0], [y0, y0], color=INK, lw=2.0, alpha=alpha)
    ax.plot([x0] * 2, [y0 - 0.08, y0 + 0.08], color=INK, lw=1.0, alpha=alpha)
    ax.plot([x0 + 1.0] * 2, [y0 - 0.08, y0 + 0.08], color=INK, lw=1.0, alpha=alpha)
    ax.text(x0 + 1.15, y0, label, family=MONO, size=8, color=INK, va="center", alpha=alpha)


CAPS = [
    "Same ratios, same orbit, wildly different sizes and clocks. In natural time units, τ = √(L³ / GM), they move in lockstep.",
]


def draw(cap_i, t_star, t_ball, t_plain, clock, clock_lab, flash=False, plain_alpha=0.0, units_mode="tau"):
    fig = new_fig(3, "Scale and time: Burrau's problem at any size", CAPS[cap_i])
    rects = [[0.04, 0.28, 0.28, 0.43], [0.36, 0.28, 0.28, 0.43], [0.68, 0.28, 0.28, 0.43]]
    axs = [fig.add_axes(r) for r in rects]
    panel(axs[0], "cosmic", t_star)
    ruler(axs[0], "1 AU")
    panel(axs[1], "balls", t_ball)
    ruler(axs[1], "10 cm")
    if plain_alpha > 0.01:
        panel(axs[2], "plain", t_plain, plain_alpha)
        ruler(axs[2], "1 unit", plain_alpha)
    else:
        bare_axes(axs[2])
        axs[2].set_facecolor(BG)

    heads = [
        (0.04, "SUN, EARTH, MOON", "pictured as Moon 3 · Earth 4 · Sun 5", "masses 3 : 4 : 5, distances 3 : 4 : 5", 1.0),
        (0.36, "FOOTBALL, TENNIS, PING-PONG", "pictured as ping-pong 3 · tennis 4 · football 5",
         "masses 3 : 4 : 5, distances 3 : 4 : 5", 1.0),
        (0.68, "NO UNITS", "just the ratios", "masses 3 : 4 : 5, distances 3 : 4 : 5", plain_alpha),
    ]
    for x, tt, s1, s2, al in heads:
        if al > 0.01:
            fig.text(x, 0.79, tt, family=MONO, size=9.5, weight="bold", color=INK, alpha=al, va="center")
            fig.text(x, 0.764, s1, size=8, color=GREY, alpha=al, va="center")
            fig.text(x, 0.742, s2, size=8, color=ACC, alpha=al, va="center")

    fig.text(0.96, 0.885, clock, family=MONO, size=15, color=ACC if flash else INK, ha="right", va="center")
    fig.text(0.96, 0.853, clock_lab, family=MONO, size=7.5, color=GREY, ha="right", va="center")

    def lines(x, l1, l2, l3, l4, al=1.0):
        for yy, tx, cc in ((0.215, l1, INK), (0.18, l2, INK), (0.145, l3, GREY), (0.11, l4, GREY)):
            fig.text(x, yy, tx, family=MONO, size=9.5 if cc == INK else 8.5, color=cc, va="center", alpha=al)

    ts, tb = t_star * TAU_STAR, t_ball * TAU_BALL
    lines(0.04, f"t / τ = {t_star:5.2f}", f"t = {fmt_t(ts)}", f"τ = {fmt_tau(TAU_STAR)}",
          "unit: 1 solar mass, 1 AU")
    lines(0.36, f"t / τ = {t_ball:5.2f}", f"t = {fmt_t(tb)}", f"τ = {fmt_tau(TAU_BALL)}",
          "unit: 100 g, 10 cm")
    if plain_alpha > 0.01:
        lines(0.68, f"t / τ = {t_plain:5.2f}", "no units at all", "τ = 1", "unit: 1, 1", plain_alpha)
    fig.text(0.96, 0.072, "artwork only: the real Moon : Earth : Sun mass ratio is 1 : 81 : 27,000,000. "
             "Bodies not to scale.", family=MONO, size=7, color=GREY, ha="right", va="center")
    return grab(fig)


N_RUN, HOLD_START, HOLD_END = 210, 14, 26


def build():
    """One looping run: all three panels play together from t = 0 to the end of the window."""
    frames = []
    for i in range(N_RUN + 1):
        t = T_END * i / N_RUN
        fr = draw(0, t, t, t, f"t / τ = {t:5.2f}", "NATURAL TIME UNITS", plain_alpha=1.0)
        if i == 0:
            frames += [fr] * HOLD_START
        frames.append(fr)
    frames += [frames[-1]] * HOLD_END
    return frames


if __name__ == "__main__":
    print("tau star (days)", TAU_STAR / DAY, "tau ball (hours)", TAU_BALL / HOUR, "ratio", RATIO)
    if len(sys.argv) > 1 and sys.argv[1] == "preview":
        t1 = 9.0
        draw(0, t1, t1, t1, "t / τ = 9.00", "NATURAL TIME UNITS", plain_alpha=1.0).save(prev_path("scaletime-1"))
        draw(0, 11.0, 11.0, 11.0, "t / τ = 11.00", "NATURAL TIME UNITS", plain_alpha=1.0).save(prev_path("scaletime-3"))
    else:
        fr = build()
        save_gif(fr, gif_path("scale-time"), fps=18)
        print(len(fr), "frames")
