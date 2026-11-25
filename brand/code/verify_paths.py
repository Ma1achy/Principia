#!/usr/bin/env python3
"""Checks the orbit integrals behind the pictures.

    python3 verify_paths.py REBUILT_DIR [--web REBUILT_DIR/code/web]

Pass criterion, written before the rebuilt files were compared:
  1. rebuilt traj.npz and catalog.npz equal the committed ones to 1e-6 in every coordinate. The boards
     round positions to 3 or 4 decimals, so a 1e-6 difference cannot change a picture.
  2. energy drift of every orbit, from the integration itself, is below 1e-10 (the README says "about 1e-11").
  3. tightening the tolerance of dragonfly's integration to 3e-14 moves its closure by less than 1e-9:
     the closure error (about 2e-3) is then the five-digit initial conditions, not the integrator.
  4. every catalogue orbit's initial conditions equal a value printed in Table I of Suvakov and
     Dmitrasinovic, PRL 110, 114301 (2013), as transcribed in the repo (fixtures/crossings/prl_orbits.py,
     docs/incoming/2026-10-26_belgrade_gallery/crosscheck.py), digit for digit. The figure-eight is the
     Chenciner-Montgomery one in crates/kernel/tests/derived.rs.
  5. the 36 s loop closes: the three body positions the drawing uses (in the P's units, cap height 2) at t and
     at t + 36 s differ by less than 1e-3, and a step across the seam (42 s - 1e-4 s to 42 s) moves a body no
     more than twice the largest step of the same size elsewhere in the loop, so the path has no jump.
Burrau's problem is not periodic: its closure is not a property, and its energy drift (close encounters)
is reported but not held to criterion 2.
"""
import ast, json, os, re, subprocess, sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
PYDIR = os.path.join(HERE, 'python')
TOL_PATH, TOL_ENERGY, TOL_TIGHT, TOL_LOOP = 1e-6, 1e-10, 1e-9, 1e-3


def load_sim():
    src = open(os.path.join(PYDIR, 'sim.py')).read().split("out={}")[0]
    ns = {}
    import io, contextlib
    exec(src, ns)
    return ns


def literal(path, name):
    tree = ast.parse(open(os.path.join(REPO, path)).read())
    for n in tree.body:
        if isinstance(n, ast.Assign) and getattr(n.targets[0], 'id', None) == name:
            return ast.literal_eval(n.value)


def sources():
    cat = {}
    txt = open(os.path.join(REPO, 'fixtures/crossings/prl_orbits.py')).read()
    for name, a, b, T in re.findall(r'\("([^"]+)",\s*([\d.]+),\s*([\d.]+),\s*([\d.]+),', txt):
        cat[(float(a), float(b), float(T))] = 'Table I row "%s" in fixtures/crossings/prl_orbits.py' % name
    for k, v in literal('docs/incoming/2026-10-26_belgrade_gallery/crosscheck.py', 'T1').items():
        cat.setdefault((v[0], v[1], v[2]), 'Table 1 row "%s" in docs/incoming/2026-10-26_belgrade_gallery/crosscheck.py' % k)
    return cat


def cat_literals():
    src = open(os.path.join(PYDIR, 'cat.py')).read()
    return ast.literal_eval(re.search(r"cat=(\{.*?\})\n", src, re.S).group(1).replace('\n', ' '))


def main():
    rebuilt = sys.argv[1]
    web = os.path.join(rebuilt, 'code/web')
    for i, a in enumerate(sys.argv):
        if a == '--web':
            web = sys.argv[i + 1]
    ok = True
    print(f'criteria: positions {TOL_PATH:g}, energy {TOL_ENERGY:g}, tight-tolerance closure shift {TOL_TIGHT:g}, loop {TOL_LOOP:g}\n')
    # 1 ---------------------------------------------------------------------------------------
    print('1. rebuilt against committed (max |difference| over every coordinate)')
    for f in ('traj', 'catalog'):
        a = np.load(os.path.join(rebuilt, 'code/python/data', f + '.npz'))
        b = np.load(os.path.join(PYDIR, 'data', f + '.npz'))
        assert sorted(a.files) == sorted(b.files), 'different orbit sets'
        for k in sorted(a.files):
            d = float(np.abs(a[k] - b[k]).max()) if a[k].shape == b[k].shape else float('inf')
            good = d <= TOL_PATH
            ok &= good
            print(f'   {f}.{k:<11} shape {a[k].shape}  max diff {d:.3g}  {"ok" if good else "FAIL"}')
    # 2, 3 ------------------------------------------------------------------------------------
    print('\n2. energy drift and closure, from integrating again (DOP853, rtol = atol = 1e-13)')
    from scipy.integrate import solve_ivp
    ns = load_sim()
    rhs, energy = ns['rhs'], ns['energy']

    def integ(p0, v0, m, T, rt):
        y0 = np.concatenate([np.ravel(p0), np.ravel(v0)])
        s = solve_ivp(rhs, (0, T), y0, args=(m,), method='DOP853', rtol=rt, atol=rt)
        e0, e1 = energy(y0, m), energy(s.y[:, -1], m)
        return abs((e1 - e0) / e0), float(np.abs(s.y[:6, -1] - y0[:6]).max())

    lit = cat_literals()
    src = sources()
    print(f'   {"orbit":<11} {"dE/E":>10} {"closure":>10}   initial conditions come from')
    for k, (p1, p2, T) in lit.items():
        v = np.array([p1, p2])
        de, cl = integ([[-1, 0], [1, 0], [0, 0]], [v, v, -2 * v], [1, 1, 1], T, 1e-13)
        s = src.get((p1, p2, T))
        ok &= de < TOL_ENERGY and s is not None
        print(f'   {k:<11} {de:10.2e} {cl:10.2e}   {s or "NOT FOUND IN THE REPO"}')
    x1 = np.array([-0.97000436, 0.24308753]); v3 = np.array([-0.93240737, -0.86473146])
    de, cl = integ([x1, -x1, [0, 0]], [-v3 / 2, -v3 / 2, v3], [1, 1, 1], 6.32591398, 1e-13)
    print(f'   {"eight":<11} {de:10.2e} {cl:10.2e}   figure-eight of Chenciner and Montgomery: crates/kernel/tests/derived.rs')
    ok &= de < TOL_ENERGY
    de, cl = integ([[1, 3], [-2, -1], [1, -1]], [[0, 0]] * 3, [3, 4, 5], 70, 1e-13)
    print(f'   {"burrau":<11} {de:10.2e} {"n/a":>10}   Burrau 3-4-5 problem, released from rest at (1,3), (-2,-1), (1,-1); not periodic, so no closure')
    p1, p2, T = lit['dragonfly']; v = np.array([p1, p2])
    _, c13 = integ([[-1, 0], [1, 0], [0, 0]], [v, v, -2 * v], [1, 1, 1], T, 1e-13)
    _, c14 = integ([[-1, 0], [1, 0], [0, 0]], [v, v, -2 * v], [1, 1, 1], T, 3e-14)
    good = abs(c13 - c14) < TOL_TIGHT
    ok &= good
    print(f'\n3. dragonfly closure at rtol 1e-13: {c13:.6e}; at 3e-14: {c14:.6e}; shift {abs(c13 - c14):.2e} {"ok" if good else "FAIL"}')
    print('   so the closure error is set by the five-digit initial conditions, not by the integrator.')
    # 5 ---------------------------------------------------------------------------------------
    print('\n5. the 36 s loop closes (node; the three body positions the drawing uses, in P units; eps = 1e-4 s)')
    js = r"""
const fs=require('fs'); const web=process.argv[1]; const out={};
for (const v of ['icon-light','icon-dark']) {
  delete globalThis.PrincipiaMark;
  let src=fs.readFileSync(web+'/principia-'+v+'.js','utf8').replace('const o = orb(t)','globalThis.__orb = orb; const o = orb(t)');
  new Function(src).call(globalThis);
  const M=globalThis.PrincipiaMark[v]; M.svgAt(6,{});            // runs renderVals once, which exposes orb()
  const pos=t=>__orb(t).bodies.flatMap(b=>b.p);
  const d=(a,b)=>{const x=pos(a),y=pos(b); return Math.max(...x.map((q,i)=>Math.abs(q-y[i])));};
  const eps=1e-4; let gen=0; for (let t=6.5;t<41;t+=0.37) gen=Math.max(gen,d(t,t+eps));
  let wrap=0; for (const t of [6,10.1,20,27.7,35.99]) wrap=Math.max(wrap,d(t,t+36));
  out[v]={seam: d(42-eps,42), generic: gen, wrap, wrap2: d(6,78)};
}
console.log(JSON.stringify(out));"""
    r = json.loads(subprocess.check_output(['node', '-e', js, web], text=True))
    for v, d in r.items():
        # the seam moves things no more than anywhere else, and the drawing repeats after 36 s
        good = d['seam'] is not None and d['seam'] <= 2 * d['generic'] and d['wrap'] < TOL_LOOP and d['wrap2'] < TOL_LOOP
        ok &= good
        print(f'   {v:<24} jump across the seam {d["seam"]:.3g} (largest elsewhere {d["generic"]:.3g}); body positions at t and t+36 differ by {d["wrap"]:.3g}  {"ok" if good else "FAIL"}')
    print('\n' + ('all criteria met' if ok else 'SOME CRITERIA NOT MET'))
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
