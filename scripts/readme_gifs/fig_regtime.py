"""FIG. 2 — regularised time: the same integrator and the same number of steps, two clocks (v2)."""
import sys
import numpy as np
from matplotlib.patches import Circle
from figkit import *

ECC = 0.7
ORBITS = 3
N_REG = 60
T = 2 * np.pi          # period for a = 1, GM = 1
q0 = np.array([1 - ECC, 0.0])
v0 = np.array([0.0, np.sqrt((1 + ECC) / (1 - ECC))])
E0 = 0.5 * v0 @ v0 - 1 / np.linalg.norm(q0)


def acc(q):
    return -q / np.linalg.norm(q) ** 3


def energy(q, v):
    return 0.5 * v @ v - 1 / np.linalg.norm(q)


def run(mode, dt_fixed=None):
    q, v, t = q0.copy(), v0.copy(), 0.0
    P, Tm, En = [q.copy()], [0.0], [0.0]
    while t < ORBITS * T - 1e-9:
        dt = dt_fixed if mode == "fixed" else (T / N_REG) * np.linalg.norm(q)
        dt = min(dt, ORBITS * T - t)
        v = v + 0.5 * dt * acc(q)
        q = q + dt * v
        v = v + 0.5 * dt * acc(q)
        t += dt
        P.append(q.copy())
        Tm.append(t)
        En.append(energy(q, v) - E0)
    return np.array(P), np.array(Tm), np.array(En)


PR, TR, ER = run("reg")
NSTEP = len(PR) - 1
PF, TF, EF = run("fixed", dt_fixed=ORBITS * T / NSTEP)
NSTEP = min(len(PR), len(PF)) - 1


def ellipse():
    e = np.linspace(0, 2 * np.pi, 400)
    return np.array([[np.cos(x) - ECC, np.sqrt(1 - ECC ** 2) * np.sin(x)] for x in e])


ELL = ellipse()


def panel(ax, P, k, title, sub):
    bare_axes(ax)
    ax.set_xlim(-3.0, 0.85)
    ax.set_ylim(-1.12, 1.12)
    ax.set_aspect("equal", adjustable="box")
    ax.plot(ELL[:, 0], ELL[:, 1], color=LIGHT, lw=1.3, zorder=1)
    ax.add_patch(Circle((0, 0), 0.035, fc=INK, ec=INK, zorder=6))
    pts = P[: k + 1]
    ax.plot(pts[:, 0], pts[:, 1], color=INK, lw=0.9, zorder=3, clip_on=True)
    ax.plot(pts[:, 0], pts[:, 1], "o", ms=2.6, color=INK, zorder=4, clip_on=True)
    ax.plot([P[k][0]], [P[k][1]], "o", ms=8, color=ACC, zorder=7, clip_on=True)
    if not (-3.0 < P[k][0] < 0.85 and -1.12 < P[k][1] < 1.12):
        ax.text(-2.95, -1.05, "flung off the page", family=MONO, size=8.5, color=ACC, va="bottom")



def strip(fig, rect, Tm, k, label):
    ax = fig.add_axes(rect)
    bare_axes(ax)
    ax.set_xlim(0, ORBITS * T)
    ax.set_ylim(0, 1)
    ax.plot([0, ORBITS * T], [0.35, 0.35], color=GREY, lw=0.8)
    for n in range(ORBITS + 1):
        ax.plot([n * T] * 2, [0.0, 0.7], color=LIGHT, lw=1.6)
        if n < ORBITS:
            ax.text(n * T + 0.15, 0.9, "close pass", family=MONO, size=7, color=GREY, ha="left")
    for j in range(k + 1):
        ax.plot([Tm[j]] * 2, [0.2, 0.5], color=INK, lw=0.9)
    ax.plot([Tm[k]] * 2, [0.1, 0.6], color=ACC, lw=2.2)
    ax.text(0, -0.15, label, family=MONO, size=7.5, color=GREY, va="top")


def draw(k):
    k = int(min(k, NSTEP))
    fig = new_fig(2, "Regularised time: same steps, spent where they matter",
                  "Same integrator, same number of steps. Each regularised step covers less physical "
                  "time when the bodies are close (dt = r dτ).")
    axl = fig.add_axes([0.04, 0.33, 0.44, 0.40])
    axr = fig.add_axes([0.52, 0.33, 0.44, 0.40])
    panel(axl, PF, k, "", "")
    panel(axr, PR, k, "", "")
    for x, tt, ss in ((0.04, "FIXED TIME STEP", "dt is the same everywhere, so it overshoots close passes"),
                      (0.52, "REGULARISED TIME STEP", "dt shrinks as the pair closes in")):
        fig.text(x, 0.775, tt, family=MONO, size=9.5, color=INK, va="center", weight="bold")
        fig.text(x, 0.745, ss, size=9, color=GREY, va="center")
    strip(fig, [0.04, 0.16, 0.44, 0.10], TF, k, "physical time: each tick is one step")
    strip(fig, [0.52, 0.16, 0.44, 0.10], TR, k, "physical time: each tick is one step")

    def err(e):
        pct = abs(e) / abs(E0) * 100
        return ">999%" if pct > 999 else f"{pct:4.1f}%"
    ef, er = EF[k], ER[k]
    col_f = ACC if abs(ef) / abs(E0) > 0.10 else INK
    col_r = ACC if abs(er) / abs(E0) > 0.10 else INK
    fig.text(0.04, 0.095, f"energy error {err(ef)}", family=MONO, size=10, color=col_f, va="center")
    fig.text(0.52, 0.095, f"energy error {err(er)}", family=MONO, size=10, color=col_r, va="center")
    fig.text(0.96, 0.095, f"step {k:>3} / {NSTEP}", family=MONO, size=10, color=GREY, va="center", ha="right")
    return grab(fig)


def build():
    frames = [draw(0)] * 12
    for k in range(1, NSTEP + 1):
        frames.append(draw(k))
    frames += [draw(NSTEP)] * 45
    return frames


if __name__ == "__main__":
    print("steps", NSTEP, "fixed rmax", np.linalg.norm(PF, axis=1).max().round(2),
          "reg rmax", np.linalg.norm(PR, axis=1).max().round(2),
          "final E err fixed/reg", EF[NSTEP].round(3), ER[NSTEP].round(3))
    if len(sys.argv) > 1 and sys.argv[1] == "preview":
        for k in (20, 60, NSTEP):
            draw(k).save(prev_path(f"regtime-{k}"))
    else:
        fr = build()
        save_gif(fr, gif_path("regularised-time"), fps=20)
        print(len(fr), "frames")
