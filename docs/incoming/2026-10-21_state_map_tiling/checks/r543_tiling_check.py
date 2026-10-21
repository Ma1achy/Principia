#!/usr/bin/env python3
"""R-543's numeric tests on a transcription of the state map's tiling maps (run with `python3 -I`).

The state map is not built yet, so this checks INSTRUCTION.md §5's numeric tests on a line-for-line transcription of the
prototype, `demo/state_flow.html` (version 9): `SM_TORUS`, `smTorusXY` (through `smTileXY` and `smNTheta`),
`smTorusLab`, `smWrapFn`, the `checker` and `tchecker` branches of `smImage`, the Picture lookup of `smPaint` and
`oklabToRgb`. Each test runs its passing arm on the prototype's constants and then the arm that must fire.
Standard library only; it reads no file and writes none. Wrap defaults bitwise needs the app (TASK-M7-49); here it is
checked on the transcription only, the prototype's version 8 lookup against version 9's defaults.
"""
import math

TAU = 2 * math.pi


def wrap(t):  # const wrap = t => ((t % TAU) + TAU) % TAU
    return math.fmod(math.fmod(t, TAU) + TAU, TAU)


def js_round(x):  # Math.round: half up, unlike Python's round
    return math.floor(x + 0.5)


def js_mod(a, b):  # JavaScript's %: the sign of the dividend
    return math.fmod(a, b)


# ------------------------------------------------------------------------------------------------- colour
def oklab_to_linear(L, a, b):  # oklabToRgb / torus_gam.js lin(): OKLab → linear sRGB, no clamp
    l_ = L + .3963377774 * a + .2158037573 * b
    m_ = L - .1055613458 * a - .0638541728 * b
    s_ = L - .0894841775 * a - 1.291485548 * b
    l, m, s = l_ ** 3, m_ ** 3, s_ ** 3
    return (4.0767416621 * l - 3.3077115913 * m + .2309699292 * s,
            -1.2684380046 * l + 2.6097574011 * m - .3413193965 * s,
            -.0041960863 * l - .7034186147 * m + 1.707614701 * s)


def transfer(x, clamp=True):  # oklabToRgb's g(): clamp to [0, 1], then the sRGB transfer, × 255
    if clamp:
        x = max(0.0, min(1.0, x))
    return 255 * (12.92 * x if x <= .0031308 else 1.055 * math.pow(x, 1 / 2.4) - .055)


def oklab_to_rgb(L, a, b):
    return tuple(transfer(c) for c in oklab_to_linear(L, a, b))


def css_byte(x):  # a canvas fillStyle rgb(x, …) with a fractional channel: rounded, clamped to 0..255
    return max(0, min(255, js_round(x)))


# ------------------------------------------------------------------------------------------------- charts
# The prototype's charts the Torus is offered on (smQuot: G is the cylinder; A, D, E, F, H, C read as the plane).
CHARTS = {
    'A': dict(xr=(-3, 3), yr=(3, -3), W=600, H=600, q='plane'),
    'E': dict(xr=(-math.pi, math.pi), yr=(0, math.pi), W=720, H=360, q='plane'),
    'H': dict(xr=(1 / 32, 31 / 32), yr=(0.6, -0.6), W=600, H=420, q='plane'),
    'G': dict(xr=(0, TAU), yr=(0.6, -0.6), W=720, H=360, q='cylinder'),
}
SM_TORUS = dict(L0=0.70, rL=0.12, R=0.074, r=0.037)
REPEATS = (1, 2, 3, 4, 6, 8)  # the Repeats setting; default 2


def sm_ntheta(n):  # smNTheta
    return 6 * max(1, js_round(n / 6))


def sm_tile_xy(tc, u, v, n, squashed=False, ny_bump=0):
    """smTileXY(tc, w, n, chartUnits = true) for finite (u, v); `squashed` is the arm that lays the period on the
    compactified square (smTileXY without chartUnits and 'square' tiling; chart A's compaction tanh(z/3))."""
    if squashed:
        sx, sy = math.tanh(u / 3), math.tanh(v / 3)
        return (sx + 1) / 2 * n, (sy + 1) / 2 * n
    nx = sm_ntheta(n) if tc['q'] == 'cylinder' else n
    ny = max(1, js_round(nx * tc['H'] / tc['W'])) + ny_bump
    if tc['q'] == 'cylinder':
        u = wrap(u)
    return ((u - tc['xr'][0]) / (tc['xr'][1] - tc['xr'][0]) * nx,
            (v - tc['yr'][0]) / (tc['yr'][1] - tc['yr'][0]) * ny)


def sm_torus_xy(tc, u, v, n, **arm):  # smTorusXY
    if tc['q'] == 'cylinder':
        _, Y = sm_tile_xy(tc, u, v, n, **arm)
        return wrap(u) / TAU, Y * n / max(1, js_round(sm_ntheta(n) * tc['H'] / tc['W']))
    return sm_tile_xy(tc, u, v, n, **arm)


def torus_lab_xy(X, Y, T=SM_TORUS, swap=False):  # smTorusLab, from (X, Y) in periods
    th, ph = TAU * X, TAU * Y
    if swap:  # the arm: L and chroma swapped, the tube's L amplitude r_L and its chroma radius r exchanged
        c = T['R'] + T['rL'] * math.cos(ph)
        return (T['L0'] + T['r'] * math.sin(ph), c * math.cos(th), c * math.sin(th))
    c = T['R'] + T['r'] * math.cos(ph)
    return (T['L0'] + T['rL'] * math.sin(ph), c * math.cos(th), c * math.sin(th))


def torus_lab(tc, u, v, n, T=SM_TORUS, n_map=None, **arm):
    X, Y = sm_torus_xy(tc, u, v, n if n_map is None else n_map, **arm)
    return torus_lab_xy(X, Y, T)


def dist(p, q):
    return math.sqrt(sum((x - y) ** 2 for x, y in zip(p, q)))


# ------------------------------------------------------------------------------------------------- pictures
N_IMG = 256


def sm_image(kind):
    """smImage's built-in checkerboards: 8 × 8 cells on a 256 × 256 canvas, as 8-bit texels; also the cells' OKLab."""
    cells, lab = {}, {}
    T = SM_TORUS
    for j in range(8):
        for i in range(8):
            if kind == 'checker':
                L = 0.82 if (i + j) % 2 else 0.5
                cl = (L, 0.12 * ((i + .5) / 4 - 1), 0.12 * (1 - (j + .5) / 4))
            else:  # 'tchecker'
                th, ph = TAU * (i + .5) / 8, TAU * (j + .5) / 8
                c = 0.8 * (T['R'] + T['r'] * math.cos(ph))
                cl = (T['L0'] + 0.8 * T['rL'] * math.sin(ph) + (0.07 if (i + j) % 2 else -0.07),
                      c * math.cos(th), c * math.sin(th))
            lab[i, j] = cl
            cells[i, j] = tuple(css_byte(x) for x in oklab_to_rgb(*cl))
    side = N_IMG // 8
    tex = [[cells[ix // side, iy // side] for ix in range(N_IMG)] for iy in range(N_IMG)]
    return tex, lab, cells


def sm_wrap_fn(mode):  # smWrapFn
    if mode == 'repeat':
        return lambda s: s - math.floor(s)
    if mode == 'clamp':
        return lambda s: max(0.0, min(1 - 1e-9, s))
    def mirror(s):
        s = js_mod(js_mod(s, 2) + 2, 2)
        return 2 - s if s > 1 else s
    return mirror


def picture(tex, u, v, mode='mirror', dim=True):
    """smPaint's Picture lookup (version 9): wrapped by the wrap mode, ×0.62 past the square if dim is on."""
    wr = sm_wrap_fn(mode)
    out = u < 0 or u > 1 or v < 0 or v > 1
    iu = min(N_IMG - 1, math.floor(wr(u) * N_IMG))
    iv = min(N_IMG - 1, math.floor(wr(v) * N_IMG))
    c = tex[iv][iu]
    return tuple(x * 0.62 for x in c) if (out and dim) else c


def picture_v8(tex, u, v):
    """Version 8's lookup (R-542's prototype): the triangle wave, always dimmed past the square."""
    def tri(s):
        s = js_mod(js_mod(s, 2) + 2, 2)
        return 2 - s if s > 1 else s
    out = u < 0 or u > 1 or v < 0 or v > 1
    iu = min(N_IMG - 1, math.floor(tri(u) * N_IMG))
    iv = min(N_IMG - 1, math.floor(tri(v) * N_IMG))
    c = tex[iv][iu]
    return tuple(x * 0.62 for x in c) if out else c


# ------------------------------------------------------------------------------------------------- tests
RESULTS = []


def report(name, ok, detail):
    RESULTS.append((name, ok))
    print(f"[{'PASS' if ok else 'FAIL'}] {name}: {detail}")


def seam_error(tc, n, span, steps, **arm):
    """max |C(u, v) − C(u + P, v)| and |C(u, v) − C(u, v + P_y)| over a sweep; P, P_y from the declared n."""
    nx = sm_ntheta(n) if tc['q'] == 'cylinder' else n
    P = (tc['xr'][1] - tc['xr'][0]) / (1 if tc['q'] == 'cylinder' else nx)  # the cylinder: X is θ once round
    if tc['q'] == 'cylinder':
        Py = (tc['yr'][1] - tc['yr'][0]) / n
    else:
        Py = (tc['yr'][1] - tc['yr'][0]) / max(1, js_round(n * tc['H'] / tc['W']))
    cx, cy = sum(tc['xr']) / 2, sum(tc['yr']) / 2
    hx, hy = abs(tc['xr'][1] - tc['xr'][0]) / 2, abs(tc['yr'][1] - tc['yr'][0]) / 2
    worst = 0.0
    for i in range(steps):
        for j in range(steps):
            u = cx + hx * span * (2 * (i + 0.5) / steps - 1)
            v = cy + hy * span * (2 * (j + 0.5) / steps - 1)
            c0 = torus_lab(tc, u, v, n, **arm)
            worst = max(worst, dist(c0, torus_lab(tc, u + P, v, n, **arm)),
                        dist(c0, torus_lab(tc, u, v + Py, n, **arm)))
    return worst


def test_torus_seam():
    span, steps = 20, 120  # the sweep reaches 20 half-widths from the square's centre, 10 squares out on each side
    worst = 0.0
    for cid, tc in CHARTS.items():
        for n in REPEATS:
            worst = max(worst, seam_error(tc, n, span, steps))
    report("Torus seam", worst <= 1e-12,
           f"max |C(u,v) − C(u+P,v)|, |C(u,v) − C(u,v+P_y)| = {worst:.2e} (≤ 1e-12) over charts "
           f"{', '.join(CHARTS)}, Repeats {REPEATS}, a {steps}² sweep to ±{span} half-widths (10 squares past each edge)")
    sq = max(seam_error(CHARTS['A'], n, span, 60, squashed=True) for n in REPEATS)
    report("Torus seam, arm: period in squashed units (must fire)", sq > 1e-12,
           f"max error {sq:.3e} > 1e-12 (latent chart, X = (tanh(u/3) + 1)/2·n)")
    ob = max(seam_error(CHARTS['A'], n, span, 60, n_map=n + 1) for n in REPEATS)
    report("Torus seam, arm: off-by-one in the period count, n + 1 periods mapped (must fire)", ob > 1e-12,
           f"max error {ob:.3e} > 1e-12 (latent chart)")
    oby = max(seam_error(CHARTS['E'], n, span, 60, ny_bump=1) for n in REPEATS)
    report("Torus seam, arm: off-by-one in the y period count, round(n·H/W) + 1 (must fire)", oby > 1e-12,
           f"max error {oby:.3e} > 1e-12 (tilted chart E, 720 × 360)")


def min_separation(points, cell):
    """The smallest distance between two distinct grid points' colours: a spatial hash with cubes of side `cell`,
    exact for every pair closer than `cell` (all neighbouring cubes searched); returns cell if none is closer."""
    grid = {}
    for k, p in enumerate(points):
        grid.setdefault(tuple(math.floor(x / cell) for x in p), []).append(k)
    best = cell
    for key, ks in grid.items():
        near = []
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    near += grid.get((key[0] + dx, key[1] + dy, key[2] + dz), [])
        for a in ks:
            pa = points[a]
            for b in near:
                if b > a:
                    d = dist(pa, points[b])
                    if d < best:
                        best = d
    return best


def torus_grid(N, T=SM_TORUS, swap=False):
    return [torus_lab_xy(i / N, j / N, T, swap) for i in range(N) for j in range(N)]


def min_radial_chroma(N, T=SM_TORUS, swap=False):
    """The tube clear of the L axis, read from the map's output only (no knowledge of its constants or formula): the
    signed radial chroma (a, b)·(cos θ, sin θ) at every point of an N² grid of one period. It is > 0 exactly when the
    torus never reaches the L axis; R = r gives 0, and R < r or L and chroma swapped give a negative minimum."""
    best = math.inf
    for i in range(N):
        th = TAU * i / N
        for j in range(N):
            _, a, b = torus_lab_xy(i / N, j / N, T, swap)
            best = min(best, a * math.cos(th) + b * math.sin(th))
    return best


def test_torus_one_to_one():
    N = 200
    sep, rad = min_separation(torus_grid(N), 0.01), min_radial_chroma(N)
    report("Torus one-to-one", sep > 0 and rad > 0,
           f"min OKLab separation between distinct points of a {N}² grid of one period = {sep:.3e} (> 0); "
           f"min signed radial chroma (a, b)·(cos θ, sin θ) = {rad:.4f} (> 0)")
    for label, T, swap in (("R ≤ r, R = r = 0.037", dict(SM_TORUS, R=SM_TORUS['r']), False),
                           ("R < r, R = 0.030", dict(SM_TORUS, R=0.03), False),
                           ("L and chroma swapped, r_L 0.12 and r 0.037 exchanged", SM_TORUS, True)):
        s_arm, r_arm = min_separation(torus_grid(N, T, swap), 0.01), min_radial_chroma(N, T, swap)
        report(f"Torus one-to-one, arm: {label} (must fire)", not (s_arm > 0 and r_arm > 0),
               f"grid min separation = {s_arm:.3e}; min signed radial chroma = {r_arm:.4f} (≤ 0), so the test fires")


def gamut_count(T, N=240):
    """Samples out of sRGB on an N² (θ, φ) sweep, and samples a per-pixel clamp would change. The shader path is the
    unclamped conversion. The "per-pixel clamp added" arm fires over the raised constants, as §5 intends: over in-gamut
    constants a clamp changes no output, so there is nothing for it to hide and nothing to detect."""
    bad = clipped = 0
    for i in range(N):
        for j in range(N):
            lin = oklab_to_linear(*torus_lab_xy(i / N, j / N, T))
            if any(x < -1e-9 or x > 1 + 1e-9 for x in lin):
                bad += 1
            if any(min(1.0, max(0.0, x)) != x for x in lin):
                clipped += 1
    return bad, clipped


def max_scale(T, N):
    """torus_gam.js: the largest k with (R·k, r·k) in gamut (30 bisections over [0, 3])."""
    lo, hi = 0.0, 3.0
    for _ in range(30):
        k = (lo + hi) / 2
        if gamut_count(dict(T, R=T['R'] * k, r=T['r'] * k), N)[0]:
            hi = k
        else:
            lo = k
    return lo


def test_torus_gamut():
    bad, clipped = gamut_count(SM_TORUS)
    report("Torus gamut", bad == 0 and clipped == 0,
           f"{bad} of 240² samples out of sRGB, {clipped} changed by a clamp (no clipping in the shader path)")
    k120, k240 = max_scale(SM_TORUS, 120), max_scale(SM_TORUS, 240)
    print(f"       largest in-gamut scale of (R, r) at L₀ 0.70, r_L 0.12: k = {k120:.4f} on 120² (torus_gam.js's sweep), "
          f"{k240:.4f} on 240²; the constants sit at {1 / k120:.4f} and {1 / k240:.4f} of it")
    up = dict(SM_TORUS, R=SM_TORUS['R'] * 1.10, r=SM_TORUS['r'] * 1.10)
    bad_up, clip_up = gamut_count(up)
    report("Torus gamut, arm: constants raised past the gamut, R, r × 1.10 (must fire)", bad_up > 0,
           f"{bad_up} of 240² samples out of sRGB")
    report("Torus gamut, arm: a per-pixel clamp added over the raised constants (must fire)", clip_up > 0,
           f"the clamped output reads in gamut, but {clip_up} samples differ from the unclamped conversion")


def unwired(tex, u, v, mode, dim=True):
    """The arm: a wrap setting that is not wired through, the lookup Mirror whatever is chosen."""
    return picture(tex, u, v, 'mirror', dim)


def test_wrap_arm():
    tex, _, _ = sm_image('checker')
    S = 160
    inside_diff = outside_diff = outside_n = 0
    wired_outside_diff = 0
    for i in range(S):
        for j in range(S):
            u, v = -1.5 + 4 * (i + 0.5) / S, -1.5 + 4 * (j + 0.5) / S
            rep, mir = picture(tex, u, v, 'repeat'), picture(tex, u, v, 'mirror')
            inside = 0 <= u <= 1 and 0 <= v <= 1
            if inside:
                inside_diff += rep != mir
            else:
                outside_n += 1
                outside_diff += rep != mir
                wired_outside_diff += unwired(tex, u, v, 'repeat') != unwired(tex, u, v, 'mirror')
    report("Wrap arm", inside_diff == 0 and outside_diff > 0,
           f"Repeat against Mirror on the checkerboard, a {S}² sweep of [−1.5, 2.5]²: {inside_diff} differ inside "
           f"the square, {outside_diff} of {outside_n} differ past it")
    report("Wrap arm, arm: the wrap setting not wired through, Mirror whatever is chosen (must fire)",
           wired_outside_diff == 0,
           f"Repeat then renders as Mirror: {wired_outside_diff} of {outside_n} differ past the square, so "
           f"'differs past the square' fails")
    clamp_edge = all(picture(tex, u, 0.5, 'clamp', False) == picture(tex, 0.999, 0.5, 'clamp', False) for u in (1.2, 3.7, 40.0))
    print(f"       Clamp holds the edge texel past the square (dim off): {clamp_edge}")
    edge_ok, edge_n, edge_diff, half_open_diff, half_open_n = wrap_edges(tex, picture)
    report("Wrap arm, the square's edges: identical on [0, 1)², s = 0 included; at s = 1 Repeat reads texel 0, Mirror texel N − 1",
           half_open_diff == 0 and edge_ok == edge_n and edge_diff > 0,
           f"a 161² grid of [0, 1]² with both ends (k/160): {half_open_diff} of {half_open_n} differ on [0, 1)²; on the "
           f"far edges (u = 1 or v = 1) {edge_ok} of {edge_n} read the stated texels and {edge_diff} differ between the modes")
    bad_ok = wrap_edges(tex, edge_moved)[0]
    report("Wrap arm, arm: the edge rule moved (s = 1 read as texel N − 1 under Repeat, as under Mirror) (must fire)",
           bad_ok < edge_n,
           f"{edge_n - bad_ok} of {edge_n} far-edge samples then read other texels than the stated ones, so the edge assertion fails")


def edge_moved(tex, u, v, mode='mirror', dim=True):
    """The arm: Repeat clamped below 1 at the far edge, so s = 1 reads texel N − 1 as under Mirror."""
    if mode == 'repeat':
        cl = lambda s: N_IMG - 1 if s == 1 else min(N_IMG - 1, math.floor((s - math.floor(s)) * N_IMG))
        c = tex[cl(v)][cl(u)]
        out = u < 0 or u > 1 or v < 0 or v > 1
        return tuple(x * 0.62 for x in c) if (out and dim) else c
    return picture(tex, u, v, mode, dim)


def wrap_edges(tex, look):
    """[0, 1]² with both ends: on [0, 1)² Repeat and Mirror agree; at s = 1 (inside the square, SM8) Repeat reads texel 0
    and Mirror texel N − 1 along that axis, the other axis as usual."""
    K = 160
    idx = lambda s, far: far if s == 1 else min(N_IMG - 1, math.floor(s * N_IMG))
    edge_ok = edge_n = edge_diff = half_open_diff = half_open_n = 0
    for k in range(K + 1):
        for l in range(K + 1):
            u, v = k / K, l / K
            rep, mir = look(tex, u, v, 'repeat'), look(tex, u, v, 'mirror')
            if u < 1 and v < 1:
                half_open_n += 1
                half_open_diff += rep != mir
                continue
            edge_n += 1
            edge_diff += rep != mir
            edge_ok += rep == tex[idx(v, 0)][idx(u, 0)] and mir == tex[idx(v, N_IMG - 1)][idx(u, N_IMG - 1)]
    return edge_ok, edge_n, edge_diff, half_open_diff, half_open_n


def test_wrap_defaults_transcription():
    tex, _, _ = sm_image('checker')
    S = 160
    diff = flip_mode = flip_dim = 0
    for i in range(S):
        for j in range(S):
            u, v = -1.5 + 4 * (i + 0.5) / S, -1.5 + 4 * (j + 0.5) / S
            old = picture_v8(tex, u, v)
            diff += picture(tex, u, v) != old
            flip_mode += picture(tex, u, v, 'repeat', True) != old
            flip_dim += picture(tex, u, v, 'mirror', False) != old
    report("Wrap defaults bitwise (transcription only; the app's test is TASK-M7-49's)", diff == 0,
           f"version 9's lookup at its defaults (Mirror, dim on) against version 8's: {diff} of {S}² texels differ")
    report("Wrap defaults, arm: the default wrap flipped to Repeat (must fire)", flip_mode > 0,
           f"{flip_mode} of {S}² differ")
    report("Wrap defaults, arm: the default dim flipped off (must fire)", flip_dim > 0, f"{flip_dim} of {S}² differ")


def seam_steps(lab):
    """Per seam: parity opposite across both seams, and along x the OKLab step across the seam against every
    neighbouring step inside its row (along y the step varies with φ; the caller holds it to the picture's rule)."""
    par_ok = True
    worst_x = 0.0
    for j in range(8):  # the x seam: cell (7, j) | the repeat's cell (0, j)
        par_ok &= (7 + j) % 2 != (0 + j) % 2
        inside = [dist(lab[i, j], lab[i + 1, j]) for i in range(7)]
        worst_x = max(worst_x, max(abs(dist(lab[7, j], lab[0, j]) - s) for s in inside))
    for i in range(8):  # the y seam: cell (i, 7) | the repeat's cell (i, 0)
        par_ok &= (i + 7) % 2 != (i + 0) % 2
    return par_ok, worst_x


def tchecker_rule(i, j):  # the tileable checkerboard's rule at any integer cell, the picture's own formula
    T = SM_TORUS
    th, ph = TAU * (i + .5) / 8, TAU * (j + .5) / 8
    c = 0.8 * (T['R'] + T['r'] * math.cos(ph))
    return (T['L0'] + 0.8 * T['rL'] * math.sin(ph) + (0.07 if (i + j) % 2 else -0.07), c * math.cos(th), c * math.sin(th))


def checker_rule(i, j):
    return (0.82 if (i + j) % 2 else 0.5, 0.12 * ((i + .5) / 4 - 1), 0.12 * (1 - (j + .5) / 4))


def test_tileable_checker_seam():
    tex, lab, cells = sm_image('tchecker')
    par_ok, worst_x = seam_steps(lab)
    # along y the step varies with φ, so the seam step is held to the rule's own step from cell 7 to cell 8
    worst_y = max(abs(dist(lab[i, 7], lab[i, 0]) - dist(tchecker_rule(i, 7), tchecker_rule(i, 8))) for i in range(8))
    # and through the 8-bit texture under Repeat: the texel across the seam is the picture's cell 0, 1 texel on
    tex_ok = all(picture(tex, 1 + 0.5 / N_IMG, (j + .5) / 8, 'repeat', False) == cells[0, j] and
                 picture(tex, (i + .5) / 8, 1 + 0.5 / N_IMG, 'repeat', False) == cells[i, 0] for i in range(8) for j in range(8))
    report("Tileable checker seam", par_ok and worst_x < 1e-12 and worst_y < 1e-12 and tex_ok,
           f"parity opposite across every seam: {par_ok}; |step across − step inside| ≤ {max(worst_x, worst_y):.1e} "
           f"in OKLab (x: every neighbour pair in the row; y: the rule's step from cell 7 to 8); Repeat's texel "
           f"past each edge is the picture's cell 0: {tex_ok}")
    lin = [oklab_to_linear(*lab[k]) for k in lab]
    at_limit = sum(any(x <= 0 or x >= 1 for x in c) for c in lin)
    byte_limit = sum(any(x in (0, 255) for x in cells[k]) for k in cells)
    lo, hi = min(min(c) for c in lin), max(max(c) for c in lin)
    report("Tileable checker in gamut (§2.3: 0 of 64 cells at a channel limit)", at_limit == 0 and byte_limit == 0,
           f"{at_limit} of 64 cells at a linear channel limit, {byte_limit} with an 8-bit channel at 0 or 255; "
           f"linear channels in [{lo:.4f}, {hi:.4f}]")
    _, olab, _ = sm_image('checker')
    o_par, o_worst_x = seam_steps(olab)
    o_worst_y = max(abs(dist(olab[i, 7], olab[i, 0]) - dist(checker_rule(i, 7), checker_rule(i, 8))) for i in range(8))
    step_in = dist(olab[0, 0], olab[1, 0])
    step_seam = dist(olab[7, 0], olab[0, 0])
    report("Tileable checker seam, arm: the old checkerboard silently substituted (must fire)",
           o_worst_x > 1e-12 or o_worst_y > 1e-12,
           f"parity still opposite: {o_par}, but the step across the x seam is {step_seam:.4f} against {step_in:.4f} "
           f"inside (|Δ| up to {max(o_worst_x, o_worst_y):.4f}): its colour ramp jumps at the seam")


def main():
    print("R-543 tiling check: a transcription of demo/state_flow.html (version 9), run with python3 -I")
    print(f"SM_TORUS = {SM_TORUS}; Repeats {REPEATS}, default 2")
    test_torus_seam()
    test_torus_one_to_one()
    test_torus_gamut()
    test_wrap_arm()
    test_wrap_defaults_transcription()
    test_tileable_checker_seam()
    failed = [n for n, ok in RESULTS if not ok]
    print(f"{len(RESULTS)} checks, {len(failed)} failed" + (": " + "; ".join(failed) if failed else ""))
    raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
    main()
