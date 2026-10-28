"""FIG. 1 — symmetry reduction: 15 numbers down to the 8D manifold (v2)."""
import sys
import numpy as np
from matplotlib.patches import Circle, Polygon
from figkit import *

M0 = np.array([1.8, 1.2, 0.9])
_P = np.array([[0.7, 1.55], [2.75, 0.1], [1.3, -0.45]])
_w = np.array([1.8, 1.2, 0.9]) / 3.9
_c = (_w[:, None] * _P).sum(0)
P0 = np.array([1.05, 0.5]) + 1.55 * (_P - _c)
V0 = 1.7 * np.array([[0.55, -0.1], [-0.2, 0.75], [0.05, 0.35]]) + np.array([0.5, 0.32])

W0 = M0 / M0.sum()
com0 = (W0[:, None] * P0).sum(0)
vcom0 = (W0[:, None] * V0).sum(0)
ref0 = P0[1] - P0[0]
theta0 = np.arctan2(ref0[1], ref0[0])
R0 = np.sqrt((W0 * ((P0 - com0) ** 2).sum(1)).sum())

STAGES = [
    ("Centre-of-mass frame", "translation and boost together: the origin rides along with the centre of mass"),
    ("Rotation", "turn the axes until body 1 to body 2 points along +x"),
    ("Mass", "only the ratios of the masses matter"),
    ("Scale", "size and time rescale together"),
]
COUNTS = [15, 11, 10, 9, 8]
MS_FRAMES = [72, 36, 36, 36]


def back(x, c1=1.25):
    x = float(np.clip(x, 0, 1))
    c3 = c1 + 1
    return 1 + c3 * (x - 1) ** 3 + c1 * (x - 1) ** 2


def rot(a):
    c, s = np.cos(a), np.sin(a)
    return np.array([[c, -s], [s, c]])


def avec(stage, p):
    """Eased progress vector [translate, boost, rotate, mass, scale]."""
    a = np.zeros(5)
    for k in range(1, stage):
        if k == 1:
            a[0] = a[1] = 1.0
        else:
            a[k] = 1.0
    if stage == 1:
        a[0] = ease(p / 0.55)
        a[1] = ease((p - 0.55) / 0.45)
    elif stage == 2:
        a[2] = back(p)
    elif stage == 3:
        a[3] = ease(p)
    elif stage == 4:
        a[4] = back(p)
    elif stage >= 5:
        a[:] = 1.0
    return a


def state(a):
    c = a[0] * com0
    phi = a[2] * theta0
    f = (1 - a[4]) + a[4] / R0
    Rm = rot(-phi)
    S = f * (P0 - c) @ Rm.T
    # mass gauge: m -> mu*m keeps the same orbit only if v -> sqrt(mu)*v (time rescales by mu^-1/2)
    mu = (1 - a[3]) + a[3] / M0.sum()
    Vrel = f ** -0.5 * mu ** 0.5 * (V0 - vcom0) @ Rm.T
    Vc = f ** -0.5 * mu ** 0.5 * (vcom0 * (1 - a[1])) @ Rm.T
    m = M0 * mu
    comS = f * (com0 * (1 - a[0])) @ Rm.T
    oldo = f * (-c) @ Rm.T
    return dict(S=S, Vrel=Vrel, Vc=Vc, m=m, comS=comS, oldo=oldo, f=f, Rm=Rm, c=c)


def angles(S):
    out = []
    for i in range(3):
        u = S[(i + 1) % 3] - S[i]
        v = S[(i + 2) % 3] - S[i]
        out.append(np.degrees(np.arccos(np.dot(u, v) / np.linalg.norm(u) / np.linalg.norm(v))))
    return out


def fmt(x):
    return f"{x:+.2f}" if abs(x) > 0.004 else " 0.00"


def draw(stage, p, counter):
    a = avec(stage, p)
    st = state(a)
    S, m = st["S"], st["m"]
    final = stage >= 5
    cap = ("Everything that cannot change the physics is quotiented out. The triangle's shape never moves."
           if not final else
           "Energy and angular momentum stay free coordinates of the manifold. They are not constraints.")
    fig = new_fig(1, "Symmetry reduction: 15 numbers to 8", cap)

    ax = fig.add_axes([0.04, 0.10, 0.56, 0.69])
    bare_axes(ax)
    ax.set_xlim(-3.6, 3.6)
    ax.set_ylim(-2.6, 2.6)
    ax.set_aspect("equal", adjustable="box")

    # lab-frame grid (moves with the world)
    for n in range(-9, 10):
        for (p0, p1) in (((n, -9), (n, 9)), ((-9, n), (9, n))):
            q0 = st["f"] * (np.array(p0) - st["c"]) @ st["Rm"].T
            q1 = st["f"] * (np.array(p1) - st["c"]) @ st["Rm"].T
            ax.plot([q0[0], q1[0]], [q0[1], q1[1]], color=LIGHT, lw=0.7 if n else 1.1, zorder=1,
                    alpha=max(0.0, 1.0 - 0.97 * ease(a[3]) - (1.0 if a[3] >= 1 else 0.0)))
    # chosen frame axes (fixed on screen)
    ax.annotate("", xy=(3.45, 0), xytext=(-3.45, 0),
                arrowprops=dict(arrowstyle="-|>", color=GREY, lw=0.9, mutation_scale=7), zorder=2)
    ax.annotate("", xy=(0, 2.5), xytext=(0, -2.5),
                arrowprops=dict(arrowstyle="-|>", color=GREY, lw=0.9, mutation_scale=7), zorder=2)
    ax.text(3.42, -0.14, "x", family=MONO, size=8, color=GREY, ha="right", va="top")
    ax.text(0.08, 2.46, "y", family=MONO, size=8, color=GREY, va="top")
    if stage >= 4 and not final:
        pass
    # unit circle for the scale gauge
    if stage >= 4:
        ax.add_patch(Circle((0, 0), 1.0, fill=False, ls=(0, (3, 3)), ec=GREY, lw=1.0, zorder=2))
    # old origin marker
    oo = st["oldo"]
    if stage <= 2 and abs(oo[0]) < 3.4 and abs(oo[1]) < 2.5 and np.linalg.norm(oo) > 0.05:
        ax.add_patch(Circle(oo, 0.07, fill=False, ec=GREY, lw=1.0, zorder=3))
        ax.text(oo[0] + 0.1, oo[1] - 0.1, "old origin", family=MONO, size=7, color=GREY, va="top", zorder=3)

    # ghost of the stage start + motion trails
    if 1 <= stage <= 4 and 0 < p < 1:
        a0 = avec(stage, 0.0)
        g = state(a0)["S"]
        ax.add_patch(Polygon(g, closed=True, fill=False, ec=GREY, lw=0.9, ls=(0, (2, 3)), zorder=3))
        for i in range(3):
            pts = np.array([state(avec(stage, p * j / 14))["S"][i] for j in range(15)])
            ax.plot(pts[:, 0], pts[:, 1], color=GREY, lw=1.0, alpha=0.85, zorder=3)

    # the triangle
    ax.add_patch(Polygon(S, closed=True, fc=ACC, ec=ACC, lw=1.6, alpha=0.10, zorder=4))
    ax.add_patch(Polygon(S, closed=True, fill=False, ec=ACC, lw=1.6, zorder=5))

    # velocities: shared drift (accent) then motion relative to the centre of mass (ink)
    kv = 0.85
    for i in range(3):
        base = S[i]
        mid = base + st["Vc"] * kv
        if np.linalg.norm(st["Vc"]) > 0.03:
            ax.annotate("", xy=mid, xytext=base,
                        arrowprops=dict(arrowstyle="-|>", color=ACC, lw=1.4, shrinkA=0, shrinkB=0,
                                        mutation_scale=8), zorder=6)
        ax.annotate("", xy=mid + st["Vrel"][i] * kv, xytext=mid,
                    arrowprops=dict(arrowstyle="-|>", color=INK, lw=1.2, shrinkA=0, shrinkB=0,
                                    mutation_scale=8), zorder=7)
    # bodies
    for i in range(3):
        r = 0.17 * np.sqrt(m[i])
        ax.add_patch(Circle(S[i], r, fc=INK, ec=INK, zorder=8))
        ax.text(S[i][0] - r - 0.06, S[i][1] + r + 0.03, str(i + 1), family=MONO, size=9,
                color=INK, ha="right", va="bottom", zorder=9)
    cs = st["comS"]
    ax.add_patch(Circle(cs, 0.1, fill=False, ec=INK, lw=1.3, zorder=9))
    ax.plot([cs[0] - 0.16, cs[0] + 0.16], [cs[1]] * 2, color=INK, lw=1.0, zorder=9)
    ax.plot([cs[0]] * 2, [cs[1] - 0.16, cs[1] + 0.16], color=INK, lw=1.0, zorder=9)

    # legend
    lx, ly = -3.5, 2.42
    ax.annotate("", xy=(lx + 0.35, ly), xytext=(lx, ly),
                arrowprops=dict(arrowstyle="-|>", color=ACC, lw=1.4, mutation_scale=8))
    ax.text(lx + 0.45, ly, "drift shared by all three bodies", size=8, color=GREY, va="center")
    ax.annotate("", xy=(lx + 0.35, ly - 0.2), xytext=(lx, ly - 0.2),
                arrowprops=dict(arrowstyle="-|>", color=INK, lw=1.2, mutation_scale=8))
    ax.text(lx + 0.45, ly - 0.2, "motion relative to the centre of mass", size=8, color=GREY, va="center")

    # ---------- right panel ----------
    x0 = 0.665
    fig.text(x0, 0.735, str(counter), size=54, weight="bold", color=INK, va="center", ha="left")
    xs = x0 + (0.115 if counter >= 10 else 0.082)
    fig.text(xs, 0.755, "dimensions" if final else "numbers describe", size=10, color=GREY, va="center")
    fig.text(xs, 0.72, "the manifold" if final else "the system", size=10, color=GREY, va="center")

    if stage == 0:
        head, nm, ds = "THE SYSTEM, AS GIVEN", "Any origin, any axes", "any size, any drift"
    elif final:
        head, nm, ds = "A POINT IN THE MANIFOLD", "6 shape and motion", "plus 2 mass ratios"
    else:
        head = f"STEP {stage} OF 4"
        nm = STAGES[stage - 1][0]
        ds = STAGES[stage - 1][1]
    fig.text(x0, 0.645, head, family=MONO, size=8, color=ACC, va="center")
    fig.text(x0, 0.612, nm, size=13, weight="bold", color=INK, va="center")
    import textwrap
    fig.text(x0, 0.585, "\n".join(textwrap.wrap(ds, 44)), size=8.5, color=GREY, va="top", linespacing=1.3)

    # readouts
    ang = angles(S)
    ref = S[1] - S[0]
    orient = np.degrees(np.arctan2(ref[1], ref[0]))
    size_now = st["f"] * R0
    rows = [
        (1, "centre of mass", f"({fmt(cs[0])}, {fmt(cs[1])})"),
        (1, "drift velocity", f"({fmt(st['Vc'][0])}, {fmt(st['Vc'][1])})"),
        (2, "orientation", f"{orient:+6.1f}°" if abs(orient) > 0.05 else "   0.0°"),
        (3, "total mass", f"{m.sum():5.2f}"),
        (4, "size", f"{size_now:5.2f}"),
    ]
    y = 0.485
    fig.text(x0, y + 0.012, "CHOSEN BY YOUR COORDINATES", family=MONO, size=7.5, color=GREY, va="center")
    y -= 0.045
    for k, lab, val in rows:
        done = stage > k or final
        active = stage == k
        col = ACC if active else (GREY if done else INK)
        fig.text(x0, y, lab, size=9.5, color=col, va="center")
        fig.text(0.955, y, val, family=MONO, size=9.5, color=col, va="center", ha="right")
        y -= 0.04
    y -= 0.012
    fig.add_artist(Line2D([x0, 0.955], [y + 0.026, y + 0.026], color=LIGHT, lw=0.9))
    fig.text(x0, y, "NEVER CHANGES", family=MONO, size=7.5, color=GREY, va="center")
    y -= 0.038
    m_n = M0 / M0.min()
    fig.text(x0, y, "triangle angles", size=9.5, color=INK, va="center")
    fig.text(0.955, y, f"{ang[0]:.0f}°  {ang[1]:.0f}°  {ang[2]:.0f}°", family=MONO, size=9.5, color=INK,
             va="center", ha="right")
    y -= 0.04
    fig.text(x0, y, "mass ratios", size=9.5, color=INK, va="center")
    fig.text(0.955, y, f"{m_n[0]:.1f} : {m_n[1]:.1f} : {m_n[2]:.1f}", family=MONO, size=9.5, color=INK,
             va="center", ha="right")
    return grab(fig)


def build():
    frames = []
    for _ in range(22):
        frames.append(draw(0, 0.0, COUNTS[0]))
    for stage in range(1, 5):
        n = MS_FRAMES[stage - 1]
        for i in range(n):
            p = (i + 1) / n
            cnt = COUNTS[stage] if p >= 0.999 else COUNTS[stage - 1]
            frames.append(draw(stage, p, cnt))
        for _ in range(14):
            frames.append(draw(stage, 1.0, COUNTS[stage]))
    for _ in range(50):
        frames.append(draw(5, 1.0, COUNTS[4]))
    return frames


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "preview":
        draw(0, 0.0, 15).save(prev_path("symmetry-a0"))
        draw(1, 0.3, 15).save(prev_path("symmetry-a1"))
        draw(1, 0.8, 15).save(prev_path("symmetry-a1b"))
        draw(2, 0.5, 11).save(prev_path("symmetry-a2"))
        draw(4, 0.5, 9).save(prev_path("symmetry-a4"))
        draw(5, 1.0, 8).save(prev_path("symmetry-af"))
    else:
        fr = build()
        save_gif(fr, gif_path("symmetry-reduction"), fps=20)
        print(len(fr), "frames")
