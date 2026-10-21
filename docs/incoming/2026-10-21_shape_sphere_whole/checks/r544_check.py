#!/usr/bin/env python3
"""R-544's check: the part of INSTRUCTION.md §4 that is pure math, on a transcription of the corpus's formulas.

Run with `python3 -I`. Standard library only; it reads no file and writes none. It is not the app's test: the app's
decode, pick, trace, scheduler and inspector are not built to R-544 yet, and the tasks R-544 names own those tests
(TASK-M2-43, TASK-M2-28, TASK-M8-44, TASK-M8-55, TASK-M8-56, TASK-M8-58, TASK-M8-67). Each check runs its passing arm,
then the arm that must fire, and prints both. §4's "Fold saving is exact" row is withdrawn with the saving (the reply:
"No folding of any kind on the shape sphere chart, visible or not"); check 6 is its replacement, and fires on a fold
left in.

Transcribed (formulas, not code):
- chart_reference §3.3, R-14's spherical map: θ = 2π·s, φ = π·(1 − t), n = (sin φ cos θ, sin φ sin θ, cos φ);
- chart_reference §3.2, the closed-form inverse (I = 1, φ_f = 0), with §3.2's small-of-a-and-b form;
- chart_reference §3.1, shape_vec: n = ((a − b)/I, 2p/I, 2q/I), q = ρ̃ ∧ λ̃;
- chart_reference §0.2 (unweight, positions) and §0.3 (momenta), and §0.4's mirror test (mirror iff λ̃_y < −δ_λ);
- R-537 EX7(b)'s quad maps at level L (θ period, the two pole maps, and R-412's equator fold for the control);
- a plain velocity-Verlet integrator (G = 1), standing in for the kernel's: only its mirror-exactness is used, and that
  holds for any integrator built from +, −, ×, ÷ and √ on the coordinates, since negation commutes with each exactly.
"""
import math

PI = math.pi
TAU = 2 * math.pi
DELTA_LAMBDA = 1e-12  # δ_λ (chart_reference §0.4, R-82)
BOUND = 1e-13  # the shape round-trip bound, provisional (REQ-CHART-053, R-181)

failures = []


def check(name, ok, detail):
    print(f"  [{'pass' if ok else 'FAIL'}] {name}: {detail}")
    if not ok:
        failures.append(name)


def fires(name, fired, detail):
    print(f"  [{'fires' if fired else 'SILENT'}] {name}: {detail}")
    if not fired:
        failures.append(name + " (its arm did not fire)")


# ------------------------------------------------------------------------------------------------ the chart
def sphere_point(theta, phi):  # §3.3, R-14
    return (math.sin(phi) * math.cos(theta), math.sin(phi) * math.sin(theta), math.cos(phi))


def angles(n):  # (θ, φ) of a unit vector, θ in [0, 2π)
    theta = math.atan2(n[1], n[0]) % TAU
    return theta, math.acos(max(-1.0, min(1.0, n[2])))


def inverse(n, inertia=1.0, phase=0.0):  # §3.2, the closed form; the smaller of a, b through n₁² + n₂²
    small = inertia * (n[1] ** 2 + n[2] ** 2) / (2 * (1 + abs(n[0])))
    large = inertia * (1 + abs(n[0])) / 2
    a, b = (large, small) if n[0] >= 0 else (small, large)
    p, q = inertia * n[1] / 2, inertia * n[2] / 2
    psi = phase + (0.0 if (p == 0 and q == 0) else math.atan2(q, p))  # R-437's select at the poles
    rho_t = (math.sqrt(a) * math.cos(phase), math.sqrt(a) * math.sin(phase))
    lam_t = (math.sqrt(b) * math.cos(psi), math.sqrt(b) * math.sin(psi))
    return rho_t, lam_t


def shape_vec(rho_t, lam_t):  # §3.1
    a = rho_t[0] ** 2 + rho_t[1] ** 2
    b = lam_t[0] ** 2 + lam_t[1] ** 2
    inertia = a + b
    p = rho_t[0] * lam_t[0] + rho_t[1] * lam_t[1]
    q = rho_t[0] * lam_t[1] - rho_t[1] * lam_t[0]
    n = ((a - b) / inertia, 2 * p / inertia, 2 * q / inertia)
    norm = math.sqrt(sum(c * c for c in n))
    return tuple(c / norm for c in n)


def reduced(m):
    m01 = m[0] + m[1]
    return m[0] * m[1] / m01, m[2] * m01, m01


def bodies(rho_t, lam_t, p_jac, m):  # §0.2 (positions) and §0.3 (momenta), body frame ρ̃ on +x
    mu_rho, mu_lam, m01 = reduced(m)
    rho = (rho_t[0] / math.sqrt(mu_rho), rho_t[1] / math.sqrt(mu_rho))
    lam = (lam_t[0] / math.sqrt(mu_lam), lam_t[1] / math.sqrt(mu_lam))
    r01 = (-m[2] * lam[0], -m[2] * lam[1])
    r = [(r01[0] - m[1] / m01 * rho[0], r01[1] - m[1] / m01 * rho[1]),
         (r01[0] + m[0] / m01 * rho[0], r01[1] + m[0] / m01 * rho[1]),
         (m01 * lam[0], m01 * lam[1])]
    p_rho, p_lam = (p_jac[0], p_jac[1]), (p_jac[2], p_jac[3])
    p = [(-p_rho[0] - m[0] / m01 * p_lam[0], -p_rho[1] - m[0] / m01 * p_lam[1]),
         (p_rho[0] - m[1] / m01 * p_lam[0], p_rho[1] - m[1] / m01 * p_lam[1]),
         (p_lam[0], p_lam[1])]
    return r, p


def jacobi_t(r, m):  # the mass-weighted Jacobi vectors of a state (§0.2 inverted), no canonical frame
    mu_rho, mu_lam, m01 = reduced(m)
    rho = (r[1][0] - r[0][0], r[1][1] - r[0][1])
    c01 = ((m[0] * r[0][0] + m[1] * r[1][0]) / m01, (m[0] * r[0][1] + m[1] * r[1][1]) / m01)
    lam = (r[2][0] - c01[0], r[2][1] - c01[1])
    return ((math.sqrt(mu_rho) * rho[0], math.sqrt(mu_rho) * rho[1]),
            (math.sqrt(mu_lam) * lam[0], math.sqrt(mu_lam) * lam[1]))


def mirror_state(r, p):  # y → −y on every rᵢ and pᵢ (§0.4's step 3)
    return [(x, -y) for x, y in r], [(x, -y) for x, y in p]


def decode(theta, phi, m, p_jac, gauge):
    """The pixel's system. gauge=False is R-544's decode (no mirror); gauge=True leaves R-141's in: C's mirror on the
    configuration and, as R-141's held-momentum rule has it, p_fixed mirrored where C mirrors, so C's output holds
    p_fixed in the canonical frame."""
    rho_t, lam_t = inverse(sphere_point(theta, phi))
    r, p = bodies(rho_t, lam_t, p_jac, m)
    if gauge and lam_t[1] < -DELTA_LAMBDA:
        r, _ = mirror_state(r, p)
        rho_t, lam_t = jacobi_t(r, m)
        r, p = bodies(rho_t, lam_t, p_jac, m)
    return r, p


def ang_mom(r, p):
    return sum(x * py - y * px for (x, y), (px, py) in zip(r, p))


# ------------------------------------------------------------------------------------------------ the integrator
def accel(r, m):
    a = [[0.0, 0.0] for _ in range(3)]
    dmin2 = math.inf
    for i in range(3):
        for j in range(i + 1, 3):
            dx, dy = r[j][0] - r[i][0], r[j][1] - r[i][1]
            d2 = dx * dx + dy * dy
            dmin2 = min(dmin2, d2)
            k = 1.0 / (d2 * math.sqrt(d2))
            a[i][0] += m[j] * dx * k
            a[i][1] += m[j] * dy * k
            a[j][0] -= m[i] * dx * k
            a[j][1] -= m[i] * dy * k
    return a, dmin2


def run(r, p, m, horizon=3.0, dt=1e-3, r_coll=0.02):
    """Velocity Verlet to the horizon or a close approach. Returns the outcome, t_end, L_z at t = 0, the frame's winding
    (the accumulated turn of ρ in the plane), and the end state."""
    x = [list(b) for b in r]
    v = [[pi[0] / mi, pi[1] / mi] for pi, mi in zip(p, m)]
    a, dmin2 = accel(x, m)
    lz = ang_mom(r, p)
    ang = math.atan2(x[1][1] - x[0][1], x[1][0] - x[0][0])
    winding, t, steps = 0.0, 0.0, int(round(horizon / dt))
    for k in range(steps):
        if dmin2 < r_coll * r_coll:
            return "collision", t, lz, winding, x
        for i in range(3):
            v[i][0] += 0.5 * dt * a[i][0]
            v[i][1] += 0.5 * dt * a[i][1]
            x[i][0] += dt * v[i][0]
            x[i][1] += dt * v[i][1]
        a, dmin2 = accel(x, m)
        for i in range(3):
            v[i][0] += 0.5 * dt * a[i][0]
            v[i][1] += 0.5 * dt * a[i][1]
        new = math.atan2(x[1][1] - x[0][1], x[1][0] - x[0][0])
        d = new - ang
        winding += d - TAU * round(d / TAU)
        ang = new
        t = (k + 1) * dt
    return "horizon", t, lz, winding, x


# ------------------------------------------------------------------------------------------------ the checks
def signed_decode():
    print("1. Signed decode (§4): pixel (θ, φ) over the whole sphere → its system's signed n̂ "
          "(the decode from n̂ both maps share)")
    masses = [(1 / 3, 1 / 3, 1 / 3), (0.2, 0.3, 0.5), (0.6, 0.15, 0.25)]
    worst, lower, total = 0.0, 0, 0
    gauge_misses, gauge_at_twin = 0, 0
    for m in masses:
        for i in range(73):  # θ over [0, 2π], the seam included
            theta = TAU * i / 72
            for j in range(37):  # φ over [0, π], both poles and the equator included
                phi = PI * j / 36
                n = sphere_point(theta, phi)
                r, _ = decode(theta, phi, m, (0, 0, 0, 0), gauge=False)
                got = shape_vec(*jacobi_t(r, m))
                worst = max(worst, max(abs(g - e) for g, e in zip(got, n)))
                total += 1
                if n[2] < -DELTA_LAMBDA:
                    lower += 1
                rg, _ = decode(theta, phi, m, (0, 0, 0, 0), gauge=True)
                gg = shape_vec(*jacobi_t(rg, m))
                if max(abs(g - e) for g, e in zip(gg, n)) > BOUND:
                    gauge_misses += 1
                    twin = sphere_point(theta, PI - phi)
                    if max(abs(g - e) for g, e in zip(gg, twin)) <= BOUND:
                        gauge_at_twin += 1
    check("signed n̂ equals the pixel's point", worst <= BOUND,
          f"{total} pixels (3 mass sets × 73 θ × 37 φ, {lower} below the equator), worst |Δn| = {worst:.2e} ≤ {BOUND:g}")
    fires("the gauge left in", gauge_misses == lower and gauge_at_twin == lower,
          f"{gauge_misses} pixels miss their point, {gauge_at_twin} of them land at (θ, π − φ); {lower} lower pixels")


def classify(held_p=None, held_lz=None):
    """Item 5's classifier on the chart's fixed coordinates: the mirror maps the system at (θ, φ) holding Jacobi momentum
    (p_ρx, p_ρy, p_λx, p_λy), in the frame ρ̃ on +x, to the system at (θ, π − φ) holding (p_ρx, −p_ρy, p_λx, −p_λy); a
    held L_z goes to −L_z. The chart is mirror-equivariant iff every held coordinate is its own image."""
    if held_lz is not None and held_lz != -held_lz:
        return "not mirror-equivariant"
    if held_p is not None and (held_p[1] != -held_p[1] or held_p[3] != -held_p[3]):
        return "not mirror-equivariant"
    return "mirror-equivariant"


PAIRS = [(0.7, 1.1), (2.3, 0.6), (4.0, 1.35), (5.5, 0.9), (1.9, 1.5)]  # (θ, φ), φ < π/2; the twin at π − φ
MASSES = (0.2, 0.3, 0.5)


def mirror_symmetry():
    print("2. Mirror symmetry (§4): released from rest, and a held momentum with no y-component (also equivariant)")
    for label, held in (("at rest", (0.0, 0.0, 0.0, 0.0)), ("p = (0.15, 0, −0.1, 0)", (0.15, 0.0, -0.1, 0.0))):
        kind = classify(held_p=held)
        check(f"{label}: classified", kind == "mirror-equivariant", kind)
        same_fate = same_t = opposite_lz = opposite_w = mirrored_end = 0
        for theta, phi in PAIRS:
            up = run(*decode(theta, phi, MASSES, held, gauge=False), MASSES)
            dn = run(*decode(theta, PI - phi, MASSES, held, gauge=False), MASSES)
            twin = decode(theta, phi, MASSES, held, gauge=False)
            # The lower pixel's system is the upper one's mirror, to rounding of the decode (sin(π − φ) ≠ sin φ in
            # the last bit); its run from that IC is then compared with the mirrored run.
            mr, mp = mirror_state(*twin)
            exact = run(mr, mp, MASSES)
            same_fate += up[0] == dn[0]
            same_t += abs(up[1] - dn[1]) <= 1e-9 and up[1] == exact[1]
            opposite_lz += abs(up[2] + dn[2]) <= 1e-12 and up[2] == -exact[2]
            opposite_w += abs(up[3] + dn[3]) <= 1e-6 and up[3] == -exact[3]
            mirrored_end += all(a[0] == b[0] and a[1] == -b[1] for a, b in zip(up[4], exact[4]))
        k = len(PAIRS)
        check(f"{label}: outcome equal at (θ, φ) and (θ, π − φ)", same_fate == k, f"{same_fate} of {k}")
        check(f"{label}: t_end equal (bitwise on the exactly mirrored IC, to rounding from the decode)", same_t == k,
              f"{same_t} of {k}")
        check(f"{label}: L_z opposite", opposite_lz == k, f"{opposite_lz} of {k}")
        check(f"{label}: winding opposite", opposite_w == k, f"{opposite_w} of {k}")
        check(f"{label}: the mirrored run is the run mirrored, bitwise", mirrored_end == k, f"{mirrored_end} of {k}")
    # The arms: the gauge left in decodes the lower pixel to the upper system, so L_z and winding are equal, not
    # opposite; a momentum rule that turns the held momentum's x-components below the equator breaks the outcome.
    held = (0.15, 0.0, -0.1, 0.0)
    eq = 0
    for theta, phi in PAIRS:
        up = run(*decode(theta, phi, MASSES, held, gauge=False), MASSES)
        dn = run(*decode(theta, PI - phi, MASSES, held, gauge=True), MASSES)
        eq += abs(up[2] + dn[2]) > 1e-9 or abs(up[3] + dn[3]) > 1e-6
    fires("a decoder that breaks the mirror (R-141's gauge left in)", eq == len(PAIRS),
          f"L_z or winding not opposite on {eq} of {len(PAIRS)} pairs")
    broken = 0
    for theta, phi in PAIRS:
        up = run(*decode(theta, phi, MASSES, held, gauge=False), MASSES)
        bad = (-held[0], held[1], -held[2], held[3])
        dn = run(*decode(theta, PI - phi, MASSES, bad, gauge=False), MASSES)
        broken += up[0] != dn[0] or abs(up[1] - dn[1]) > 1e-9 or abs(up[3] + dn[3]) > 1e-6
    fires("a momentum rule that breaks the mirror (p_x turned below the equator)", broken == len(PAIRS),
          f"outcome, t_end or winding broken on {broken} of {len(PAIRS)} pairs")


def mirror_control():
    print("3. Mirror control (§4): a chart holding L_z ≠ 0 fixed")
    held = (0.0, 0.0, 0.0, 0.3)  # p_λ = (0, 0.3): L_z = λ_x·0.3 at both pixels, the same sign
    kind = classify(held_p=held)
    kind_lz = classify(held_lz=0.3)
    check("classified", kind == "not mirror-equivariant" and kind_lz == "not mirror-equivariant",
          f"held p_λ,y = 0.3: {kind}; held L_z = 0.3: {kind_lz}; the test asserts nothing here")
    differ = 0
    for theta, phi in PAIRS:
        up = run(*decode(theta, phi, MASSES, held, gauge=False), MASSES)
        dn = run(*decode(theta, PI - phi, MASSES, held, gauge=False), MASSES)
        differ += up[0] != dn[0] or abs(up[1] - dn[1]) > 1e-9 or abs(up[2] + dn[2]) > 1e-12
    fires("a test that passes by assuming the symmetry", differ > 0,
          f"asserting anyway fails on {differ} of {len(PAIRS)} pairs: the hemispheres are different systems")


def trace_through_syzygy():
    print("4. Trace through a syzygy (§4): the figure-eight (Chenciner–Montgomery), equal masses, G = 1")
    m = (1.0, 1.0, 1.0)
    x1 = (-0.97000436, 0.24308753)
    v3 = (-0.93240737, -0.86473146)
    r = [x1, (-x1[0], -x1[1]), (0.0, 0.0)]
    p = [(-v3[0] / 2, -v3[1] / 2), (-v3[0] / 2, -v3[1] / 2), v3]
    x = [list(b) for b in r]
    v = [list(b) for b in p]
    a, _ = accel(x, m)
    dt, ts, n3 = 1e-4, [], []
    for k in range(int(1.3 / dt) + 1):
        n3.append(shape_vec(*jacobi_t(x, m))[2])
        ts.append(k * dt)
        for i in range(3):
            v[i][0] += 0.5 * dt * a[i][0]
            v[i][1] += 0.5 * dt * a[i][1]
            x[i][0] += dt * v[i][0]
            x[i][1] += dt * v[i][1]
        a, _ = accel(x, m)
        for i in range(3):
            v[i][0] += 0.5 * dt * a[i][0]
            v[i][1] += 0.5 * dt * a[i][1]
    # The run starts at a syzygy (body 2 at the midpoint); the next crossing of n₃ = 0, after t = 0.5, is the test's.
    c = next(k for k in range(5000, len(n3) - 1) if n3[k] * n3[k + 1] <= 0)
    h = 50
    left = (n3[c] - n3[c - h]) / (h * dt)
    right = (n3[c + 1 + h] - n3[c + 1]) / (h * dt)
    signed_ok = n3[c - h] * n3[c + 1 + h] < 0 and abs(left) > 0.1 and abs(right - left) < 0.05 * abs(left)
    check("signed n̂₃ changes sign through 0, nonzero derivative, no kink", signed_ok,
          f"crossing at t ≈ {ts[c]:.4f}: dn₃/dt left {left:+.4f}, right {right:+.4f}")
    fl = [abs(z) for z in n3]
    fleft = (fl[c] - fl[c - h]) / (h * dt)
    fright = (fl[c + 1 + h] - fl[c + 1]) / (h * dt)
    fires("a folded path (|n₃|, the hemisphere fold)", fleft * fright < 0 and abs(fleft + fright) < 0.05 * abs(fleft),
          f"d|n₃|/dt left {fleft:+.4f}, right {fright:+.4f}: a bounce at the equator")


def canonical_address(ix, iy, level, equator_fold):
    """R-537 EX7(b) at level L, n = 2^L, R-14's φ = π(1 − t): the θ period and the pole maps at t = 1 and t = 0. With
    equator_fold, R-412's one-hemisphere address (the control): the fold at the equator, t = 0, iy' = −1 − iy, ix
    unchanged; R-544 withdraws it."""
    n = 2 ** level
    for _ in range(64):
        ix %= n
        if 0 <= iy < n:
            return ix, iy
        if iy >= n:
            iy, ix = 2 * n - 1 - iy, (ix + n // 2) % n
        elif equator_fold:
            iy = -1 - iy
        else:
            iy, ix = -1 - iy, (ix + n // 2) % n
    raise RuntimeError("no address")


def no_repeat_at_equator():
    print("5. No repeat at the equator (§4): the quad maps of R-537 EX7(b) at L = 1..8")
    own = twin_hits = total = pole_ok = 0
    for level in range(1, 9):
        n = 2 ** level
        for ix in range(n):
            for iy in range(n):
                total += 1
                twin = (ix, n - 1 - iy)  # the quad at (θ, π − φ)
                if canonical_address(ix, iy, level, False) == (ix, iy) and twin != (ix, iy):
                    own += 1
            # past each pole, R-14's maps: t > 1 and t < 0, θ shifted by half its period
            pole_ok += canonical_address(ix, n, level, False) == ((ix + n // 2) % n, n - 1)
            pole_ok += canonical_address(ix, -1, level, False) == ((ix + n // 2) % n, 0)
    check("every quad inside the square is its own canonical quad, not its twin's", own == total,
          f"{own} of {total}; the pole maps shift θ by half a period at both edges: {pole_ok} of "
          f"{2 * sum(2 ** l for l in range(1, 9))}")
    for level in range(1, 9):
        n = 2 ** level
        for ix in range(n):
            for iy in range(-n, 0):  # past t = 0
                twin_hits += canonical_address(ix, iy, level, True)[0] == ix
    fires("R-537's equator fold left in", twin_hits > 0,
          f"{twin_hits} quads past t = 0 fold onto the same θ (the mirror), where R-14's pole map shifts θ by π")


def no_fold():
    print("6. No fold of any kind on the chart (the reply), and 7. the area statistics (§2 item 7): an 8 × 6 grid, "
          "released from rest")
    thetas = [TAU * (i + 0.5) / 8 for i in range(8)]
    phis = [PI * (j + 0.5) / 6 for j in range(6)]  # symmetric about π/2, no equator row
    pixels = [(th, ph) for th in thetas for ph in phis]
    upper = [(th, ph) for th, ph in pixels if ph < PI / 2]

    def own(th, ph):  # the pixel's own system, integrated
        return run(*decode(th, ph, MASSES, (0, 0, 0, 0), gauge=False), MASSES, horizon=2.0)

    def twin(th, ph):
        return th, min(phis, key=lambda q: abs(q - (PI - ph)))

    # R-544's chart: every pixel integrated from its own decode, once.
    chart = {k: own(*k) for k in pixels}
    chart_runs = len(pixels)
    # The withdrawn saving, left in: the upper hemisphere integrated, the lower copied from its twins.
    folded = {k: chart[k] for k in upper}
    folded.update({twin(*k): chart[k] for k in upper})
    folded_runs = len(upper)

    def audit(results, runs):
        """What a fold left in breaks: one integration per pixel, and every pixel's result its own system's. Read on
        the signed field (the frame's winding), which a copy from the twin carries with the wrong sign; outcome and
        t_end are mirror-invariant, so they cannot see the fold here."""
        problems = []
        if runs != len(pixels):
            problems.append(f"{runs} integrations for {len(pixels)} pixels")
        wrong = sum(abs(results[k][3] - chart[k][3]) > 1e-6 for k in pixels if abs(chart[k][3]) > 1e-6)
        if wrong:
            problems.append(f"{wrong} pixels carry their twin's winding, the wrong sign")
        return problems

    got = audit(chart, chart_runs)
    check("every pixel integrated once, from its own decode", not got,
          f"{chart_runs} integrations for {len(pixels)} pixels, every winding its own" if not got else "; ".join(got))
    got = audit(folded, folded_runs)
    fires("a fold left in (the lower half copied from its twins, the withdrawn saving)", bool(got), "; ".join(got))
    whole = sum(r[0] == "collision" for r in chart.values())
    half = sum(chart[k][0] == "collision" for k in upper)
    check("area statistics: over the whole sphere each fate counted twice, equally, the fraction unchanged",
          whole == 2 * half and whole / len(pixels) == half / len(upper),
          f"collisions {whole} of {len(pixels)} over the whole sphere, {half} of {len(upper)} over the upper "
          f"hemisphere: {whole / len(pixels):.4f} = {half / len(upper):.4f}")
    folded_count = half / len(pixels)
    fires("a count folded to the upper hemisphere over the whole sphere's total", folded_count != whole / len(pixels),
          f"{half} of {len(pixels)} = {folded_count:.4f}, not {whole / len(pixels):.4f}: half the fraction")


if __name__ == "__main__":
    print("R-544's check: INSTRUCTION.md §4, the pure-math part (python3 -I)")
    signed_decode()
    mirror_symmetry()
    mirror_control()
    trace_through_syzygy()
    no_repeat_at_equator()
    no_fold()
    print(f"{'0 failed' if not failures else str(len(failures)) + ' failed: ' + '; '.join(failures)}")
    raise SystemExit(1 if failures else 0)
