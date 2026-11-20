"""Generate prl_orbits.json: ten periodic orbits of Šuvakov & Dmitrašinović, PRL 110, 114301 (2013), Table I,
integrated from the table's initial conditions, kept as the states bracketing each syzygy (each sign change of the
triangle's orientation), in our body labels (TASK-M3-16; symbolic_dynamics_contract §1; REQ-PAY-070's cross-check).

Adapted from the script RQ-632 was first read with (30 Oct 2026; its 31 Oct line). Run offline, never by CI:

    python3 -I fixtures/crossings/prl_orbits.py fixtures/crossings/prl_orbits.json

Needs numpy and scipy. Table I's initial conditions (G = m = 1): x₁ = (−1, 0), x₂ = (1, 0), x₃ = (0, 0),
v₁ = v₂ = (ẋ₁(0), ẏ₁(0)), v₃ = −2·v₁. Its body 3 starts in the middle, the Šuvakov plane's middle body, so (R-630,
item 3) ours are: body 0 = its 1, body 1 = its 3, body 2 = its 2.

Each orbit is integrated with DOP853 (rtol = atol = 1e-12) and sampled at the N + 1 times (k + 1/2)·T/N, k = 0 to N,
so no sample is the collinear start and the syzygy at t = 0 is met once, near t = T (the grid extended past N, within
0.001·T, until the march ends on the side it starts on, Table I's T being given to four decimals);
a sign change of the orientation between two grid points is a syzygy, and the two grid states are kept. Between two
kept pairs the orientation keeps its sign, so the kept states, in order, are a march with the same side changes.
"""

import json
import platform
import sys

import numpy as np
import scipy
from scipy.integrate import solve_ivp

N = 200_000

# Table I: name, ẋ₁(0), ẏ₁(0), T, the free group element as printed.
ROWS = [
    ("I.A.1 butterfly I", 0.30689, 0.12551, 6.2356, "(ab)2(AB)2", "ababABAB"),
    ("I.A.2 butterfly II", 0.39295, 0.09758, 7.0039, "(ab)2(AB)2", "ababABAB"),
    ("I.B.1 moth I", 0.46444, 0.39606, 14.8939, "(baBABabABA)", "baBABabABA"),
    ("I.B.2 moth II", 0.43917, 0.45297, 28.6703, "abABabABAbaBAbaBAB", "abABabABAbaBAbaBAB"),
    ("I.B.3 butterfly III", 0.40592, 0.23016, 13.8658, "ababABAbabaBAB", "ababABAbabaBAB"),
    ("I.B.5 goggles", 0.08330, 0.12789, 10.4668, "ababABBAbabaBAAB", "ababABBAbabaBAAB"),
    ("I.B.7 dragonfly", 0.08058, 0.58884, 21.2710, "bbABabABaaBAbaBA", "bbABabABaaBAbaBA"),
    ("II.B.1 yarn", 0.55906, 0.34919, 55.5018, "(babABabaBA)3", "babABabaBA" * 3),
    ("II.C.2a yin-yang I", 0.51394, 0.30474, 17.3284, "ababABAbaBAB", "ababABAbaBAB"),
    ("II.C.2b yin-yang I", 0.28270, 0.32721, 10.9626, "ababABAbaBAB", "ababABAbaBAB"),
]

# Our body k is Table I's body TABLE[k] (0-based).
TABLE = [0, 2, 1]


def rhs(_t, s):
    x = s[:6].reshape(3, 2)
    a = np.zeros((3, 2))
    for i in range(3):
        for j in range(3):
            if i != j:
                r = x[j] - x[i]
                a[i] += r / np.linalg.norm(r) ** 3
    return np.concatenate([s[6:], a.ravel()])


def ours(s):
    x = s[:6].reshape(3, 2)
    return [[float(x[TABLE[k]][0]), float(x[TABLE[k]][1])] for k in range(3)]


def d_of(r):
    """symbolic_dynamics_contract §1's d = −((r₁ − r₀) × (r₂ − r₀))."""
    u = (r[1][0] - r[0][0], r[1][1] - r[0][1])
    w = (r[2][0] - r[0][0], r[2][1] - r[0][1])
    return -(u[0] * w[1] - u[1] * w[0])


def between(r):
    """§1's classifier: the body opposite the longest side, the lowest on a tie."""
    def sq(i, j):
        return (r[j][0] - r[i][0]) ** 2 + (r[j][1] - r[i][1]) ** 2
    n = [sq(1, 2), sq(2, 0), sq(0, 1)]
    return max(range(3), key=lambda k: (n[k], -k))


def word(states):
    w = ""
    for s0, s1 in zip(states, states[1:]):
        d0, d1 = d_of(s0), d_of(s1)
        if (d0 < 0) != (d1 < 0):
            positive = d0 < 0
            b = between(s1 if positive else s0)
            if b == 0:
                w += "a" if positive else "A"
            elif b == 2:
                w += "b" if positive else "B"
    return w


def reduce(w):
    out = []
    for c in w:
        if out and out[-1] == c.swapcase():
            out.pop()
        else:
            out.append(c)
    while len(out) > 1 and out[0] == out[-1].swapcase():
        out = out[1:-1]
    return "".join(out)


def inverse(w):
    return w[::-1].swapcase()


def symmetry(got, published):
    """R-630 item 5's comparison: conjugacy, then the inverse, the exchange, or both."""
    p = reduce(published)
    rots = {p[i:] + p[:i] for i in range(len(p))}
    for name, f in [
        ("conjugacy", lambda w: w),
        ("inverse", inverse),
        ("exchange", lambda w: w.swapcase()),
        ("inverse and exchange", lambda w: inverse(w).swapcase()),
    ]:
        if f(got) in rots:
            return name
    return None


def main(out):
    orbits = []
    for name, vx, vy, period, printed, published in ROWS:
        s0 = np.array([-1, 0, 1, 0, 0, 0, vx, vy, vx, vy, -2 * vx, -2 * vy], float)
        sol = solve_ivp(rhs, (0, 1.001 * period), s0, method="DOP853", rtol=1e-12, atol=1e-12, dense_output=True)
        ts = (np.arange(N + 1) + 0.5) * (period / N)
        states = [ours(sol.sol(t)) for t in ts]
        # Table I's period is given to four decimals, so the syzygy at T may lie a little past the last sample: extend
        # the grid until the march ends on the side it starts on, so the sampled loop closes.
        k = N + 1
        while (d_of(states[-1]) < 0) != (d_of(states[0]) < 0):
            assert k < 1.001 * N, f"{name}: the march does not close within 0.001·T"
            states.append(ours(sol.sol((k + 0.5) * (period / N))))
            k += 1
        kept = []
        for k in range(1, len(states)):
            if (d_of(states[k - 1]) < 0) != (d_of(states[k]) < 0):
                kept += [states[k - 1], states[k]]
        least = min(abs(d_of(s)) for s in kept)
        assert least > 1e-9, f"{name}: a kept state within {least:e} of the equator"
        for a, b in zip(kept[::2], kept[1::2]):
            assert between(a) == between(b), f"{name}: the classifier differs across a syzygy's two states"
        got = reduce(word(kept))
        assert got == reduce(word(states)), f"{name}: the kept states read another word"
        sym = symmetry(got, published)
        assert sym is not None, f"{name}: {got} is not {published} up to the symmetries"
        closure = float(np.linalg.norm(sol.sol(period) - s0))
        print(f"{name}: {len(kept) // 2} syzygies, closure {closure:.1e}, word {got}, {sym}", file=sys.stderr)
        orbits.append({
            "name": name,
            "table_i": {"vx": vx, "vy": vy, "T": period, "word": printed},
            "published": published,
            "word": got,
            "symmetry": sym,
            "states": kept,
        })
    doc = {
        "about": (
            "Ten periodic orbits of Šuvakov & Dmitrašinović, PRL 110, 114301 (2013), Table I, integrated from its "
            "initial conditions (G = m = 1) with scipy's DOP853 at rtol = atol = 1e-12, sampled at the "
            f"{N} + 1 times (k + 1/2)·T/{N}, k = 0 to {N} (extended until the march ends on the side it starts on), over one period; kept are the two samples bracketing each sign change of the triangle's "
            "orientation, in order, positions only, in our body labels (ours 0, 1, 2 = Table I's 1, 3, 2; R-630 "
            "item 3), as [[x0, y0], [x1, y1], [x2, y2]]. 'published' is Table I's word written out; 'word' is the "
            "cyclically reduced word symbolic_dynamics_contract §1 reads from the kept states, and 'symmetry' the one "
            "R-630 item 5 needed to match it. Generated by fixtures/crossings/prl_orbits.py (adapted from the script "
            "RQ-632 was first read with, 30 Oct 2026; its 31 Oct line), never regenerated by CI; not transcribed from prin-rs. TASK-M3-16, "
            "REQ-PAY-070's cross-check; REQ-VAL-043 is TASK-M3-30's."
        ),
        "generator": {
            "command": "python3 -I fixtures/crossings/prl_orbits.py fixtures/crossings/prl_orbits.json",
            "python": platform.python_version(),
            "numpy": np.__version__,
            "scipy": scipy.__version__,
        },
        "orbits": orbits,
    }
    with open(out, "w") as f:
        json.dump(doc, f, indent=1, ensure_ascii=False)
        f.write("\n")


if __name__ == "__main__":
    main(sys.argv[1])
