#!/usr/bin/env python3
"""Independent check that no body or trail of the Dragonfly icon is ever inside the solid P, and that the
orbit passes through the hole 9 times per loop.

    python3 verify_clearance.py [--boards DIR] [--steps 120000]

Independent of the search scripts (which use matplotlib's point-in-polygon and their own distance code):
this uses a winding-number test and its own segment distance. Two paths are checked:
  (a) the orbit integrated again with DOP853 at rtol = atol = 1e-13 over one period in 120,000 equal steps,
      placed in 3D with dragon_P.json exactly as dragon.py places it;
  (b) what the board draws: the body positions of the committed Dragonfly board sampled 120,000 times over
      the 36 s loop (node runs the board's own orb(t)), plus every trail point at 600 times.
Pass criteria, written before running: 0 points inside the solid (|z| < 0.2, inside the outline, outside the
hole); 0 points within the body radius 0.095 of the solid; exactly 9 passes through the hole per loop.
Controls: the same check on a placement moved 0.5 in x must report contact, and a body placed in the middle of
the stem of the P must be reported inside.
"""
import json, os, subprocess, sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, 'python', 'data')
H, M_B = 0.2, 0.095


def winding(p, poly):
    """Winding number of polygon around each point of p (n,2): nonzero means inside."""
    a = poly[None, :, :] - p[:, None, :]
    b = np.roll(poly, -1, axis=0)[None, :, :] - p[:, None, :]
    ang = np.arctan2(a[..., 0] * b[..., 1] - a[..., 1] * b[..., 0], (a * b).sum(-1))
    return np.rint(ang.sum(1) / (2 * np.pi)).astype(int)


def seg_dist(p, poly):
    A = poly; B = np.roll(poly, -1, axis=0); AB = B - A
    L = (AB * AB).sum(1)
    out = np.full(len(p), np.inf)
    for i in range(0, len(p), 20000):
        q = p[i:i + 20000][:, None, :] - A[None]
        t = np.clip((q * AB).sum(-1) / L, 0, 1)
        out[i:i + 20000] = np.sqrt(((q - t[..., None] * AB) ** 2).sum(-1)).min(1)
    return out


def contacts(pts, outline, hole, margin=0.0):
    """Points (n,3) inside the slab of the solid P, grown by margin."""
    xy, z = pts[:, :2], np.abs(pts[:, 2])
    slab = z < H + margin
    cand = np.where(slab)[0]
    if not len(cand):
        return 0
    q = xy[cand]
    inside = (winding(q, outline) != 0) & (winding(q, hole) == 0) if margin == 0 else None
    if margin:
        inside = ((winding(q, outline) != 0) & (winding(q, hole) == 0)) | \
                 (np.minimum(seg_dist(q, outline), seg_dist(q, hole)) < margin)
    return int(inside.sum())


def hole_passes(W, hole):
    n = 0
    for b in range(W.shape[1]):
        z = W[:, b, 2]
        idx = np.where(np.sign(z) != np.sign(np.roll(z, -1)))[0]
        for i in idx:
            a, c = W[i, b], W[(i + 1) % len(W), b]
            t = a[2] / (a[2] - c[2]); q = a + (c - a) * t
            n += int(winding(q[None, :2], hole)[0] != 0)
    return n


def place(Y, P):
    psi, phi, cx, cy, S, tau = P
    dv = np.array([np.cos(phi), np.sin(phi), 0.0]); e = np.array([0, 0, 1.0]); k = dv
    e = e * np.cos(tau) + np.cross(k, e) * np.sin(tau) + k * (k @ e) * (1 - np.cos(tau))
    c, s = np.cos(psi), np.sin(psi)
    X = (Y[..., 0] * c - Y[..., 1] * s) * S; Yy = (Y[..., 0] * s + Y[..., 1] * c) * S
    return np.stack([cx + X * dv[0] + Yy * e[0], cy + X * dv[1] + Yy * e[1], X * dv[2] + Yy * e[2]], -1)


def integrate(steps):
    from scipy.integrate import solve_ivp
    ns = {}
    exec(open(os.path.join(HERE, 'python', 'sim.py')).read().split("def run(")[0], ns)
    p1, p2, T = 0.08058, 0.58884, 21.2710
    v = np.array([p1, p2]); y0 = np.concatenate([np.ravel([[-1, 0], [1, 0], [0, 0]]), np.ravel([v, v, -2 * v])])
    sol = solve_ivp(ns['rhs'], (0, T), y0, args=([1, 1, 1],), method='DOP853', rtol=1e-13, atol=1e-13, dense_output=True)
    tt = np.linspace(0, T, steps + 1)[:-1]
    return sol.sol(tt)[:6].T.reshape(-1, 3, 2)


NODE = r"""
const fs=require('fs'); const [file,steps]=[process.argv[1],+process.argv[2]];
let js=fs.readFileSync(file,'utf8').replace('const o = orb(t)','globalThis.__orb = orb; const o = orb(t)');
delete globalThis.PrincipiaMark; new Function(js).call(globalThis);
PrincipiaMark['icon-light'].svgAt(6,{});
const bodies=[], trails=[];
for (let k=0;k<steps;k++){ const t=36*k/steps; const o=__orb(t); for (const b of o.bodies) bodies.push(...b.p);
  if (k%(steps/600)===0) for (const tr of o.trails) for (const p of tr.pts) trails.push(...p); }
fs.writeFileSync(process.argv[3], JSON.stringify({bodies,trails}));
"""


def board_points(web_js, steps, tmp):
    out = os.path.join(tmp, 'board_points.json')
    subprocess.check_call(['node', '-e', NODE, web_js, str(steps), out])
    d = json.load(open(out))
    return np.array(d['bodies']).reshape(-1, 3), np.array(d['trails']).reshape(-1, 3)


def main():
    import argparse, tempfile
    ap = argparse.ArgumentParser()
    ap.add_argument('--web', default=os.path.join(os.path.dirname(HERE), 'code', 'web'))
    ap.add_argument('--steps', type=int, default=120000)
    a = ap.parse_args()
    g = json.load(open(os.path.join(DATA, 'pgeo.json')))
    outline, hole = np.array(g['out']), np.array(g['hole'])
    P = json.load(open(os.path.join(DATA, 'dragon_P.json')))
    ok = True
    # controls first: the test has to be able to fail
    stem = np.array([[-0.5, -0.5, 0.0]])        # inside the stem of the P, in the slab
    print('controls:')
    c1 = contacts(stem, outline, hole)
    c2 = contacts(np.array([[3.0, 3.0, 0.0]]), outline, hole)
    print(f'   a point in the stem of the P: {c1} inside (must be 1); a point far away: {c2} (must be 0)')
    ok &= c1 == 1 and c2 == 0
    Y = integrate(a.steps)
    W = place(Y, P)
    Pb = list(P); Pb[2] += 0.5
    cm = contacts(place(Y, Pb).reshape(-1, 3)[::10], outline, hole)
    print(f'   the same orbit moved 0.5 in x: {cm} of {len(W) * 3 // 10} sampled points inside the solid (must be > 0)')
    ok &= cm > 0
    print(f'\n(a) integrated orbit, {len(W)} steps x 3 bodies')
    c0 = contacts(W.reshape(-1, 3), outline, hole)
    cmg = contacts(W.reshape(-1, 3), outline, hole, M_B)
    hp = hole_passes(W, hole)
    print(f'   inside the solid: {c0}; within {M_B} of it: {cmg}; passes through the hole per period: {hp}')
    ok &= c0 == 0 and cmg == 0
    with tempfile.TemporaryDirectory() as tmp:
        bp, tp = board_points(os.path.join(a.web, 'principia-icon-light.js'), a.steps, tmp)
    print(f'\n(b) what the board draws: {len(bp)} body points over the 36 s loop, {len(tp)} trail points')
    b0 = contacts(bp, outline, hole); t0 = contacts(tp, outline, hole)
    b1 = contacts(bp, outline, hole, M_B)
    Wb = bp.reshape(-1, 3, 3)
    hpb = hole_passes(Wb, hole)
    print(f'   bodies inside the solid: {b0}; trails inside the solid: {t0}; bodies within {M_B}: {b1}; hole passes per loop: {hpb}')
    ok &= b0 == 0 and t0 == 0 and b1 == 0
    ok &= hpb == 9
    print(f'   hole passes: integrated orbit {hp} per period, board {hpb} per loop (must be 9 for the board)')
    print('\n' + ('all criteria met' if ok else 'SOME CRITERIA NOT MET'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
