import numpy as np, json
d=np.load('traj.npz'); g=json.load(open('pgeo.json'))
INK='#1d1b19'; PAPER='#efe9dc'; M='#e2236f'; Y='#f4b400'; C='#14a6cf'
def fl(a,n=3): return json.dumps([round(float(x),n) for x in np.ravel(a)]).replace(' ','')
CORE='''
    const H = 0.2, D = 7, K = 150, CX = 320, CY = 292, PITCH = 0.3, L = [-0.5, 0.55, 0.67];
    const OUT = %(out)s, HOLE = %(hole)s, CO = %(co)s, CH = %(ch)s;
    const t = ((this.state && this.state.t) || 0) * (this.props.speed ?? 1);
    const spin = %(yaw0)s + %(amp)s * Math.sin(t * 0.32);
    const rot = (p) => { const c = Math.cos(spin), s = Math.sin(spin); let x = p[0] * c + p[2] * s, z = -p[0] * s + p[2] * c, y = p[1];
      const cp = Math.cos(PITCH), sp = Math.sin(PITCH); return [x, y * cp - z * sp, y * sp + z * cp]; };
    const pj = (p) => { const v = rot(p), f = D / (D - v[2]); return [CX + K * v[0] * f, CY - K * v[1] * f, v[2], f]; };
    const ln = Math.hypot(...L), LL = L.map(x => x / ln);
    const fs = Math.sign(rot([0, 0, 1])[2]) || 1, front = (p) => p[2] * fs > 0;
    const hx = (h) => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16));
    const mix = (a, b, u) => { const A = hx(a), B = hx(b); return 'rgb(' + A.map((x, i) => Math.round(x + (B[i] - x) * u)).join(',') + ')'; };
    const P2 = (q) => q[0].toFixed(1) + ',' + q[1].toFixed(1);
    const poly = (pts) => pts.map((q, i) => (i ? 'L' : 'M') + P2(q)).join('') + 'Z';
    const circ = (q, r) => 'M' + (q[0] - r).toFixed(1) + ',' + q[1].toFixed(1) + 'a' + r.toFixed(1) + ',' + r.toFixed(1) + ' 0 1,0 ' + (2 * r).toFixed(1) + ',0a' + r.toFixed(1) + ',' + r.toFixed(1) + ' 0 1,0 ' + (-2 * r).toFixed(1) + ',0';
    const back = [], mid = [], fore = [];
    const push = (grp, d, f, s, w, o, k) => grp.push({ d, f, s, w: (+w).toFixed(2), o: (+o).toFixed(3), k });
'''
SOLID='''
    [[OUT, 1], [HOLE, 1]].forEach(([lp]) => {
      for (let i = 0; i < lp.length; i++) {
        const a = lp[i], b = lp[(i + 1) %% lp.length];
        let nx = b[1] - a[1], ny = -(b[0] - a[0]); const nl = Math.hypot(nx, ny); nx /= nl; ny /= nl;
        const nv = rot([nx, ny, 0]); if (nv[2] <= 0.01) continue;
        const lam = Math.max(0, nv[0] * LL[0] + nv[1] * LL[1] + nv[2] * LL[2]);
        const q = [pj([a[0], a[1], H]), pj([b[0], b[1], H]), pj([b[0], b[1], -H]), pj([a[0], a[1], -H])];
        const col = mix('%(s0)s', '%(s1)s', 0.15 + 0.85 * lam);
        push(mid, poly(q), col, col, 0.8, 1, (q[0][2] + q[1][2] + q[2][2] + q[3][2]) / 4);
      }
    });
    const z = fs * H;
    push(mid, poly(OUT.map(a => pj([a[0], a[1], z]))) + poly(HOLE.map(a => pj([a[0], a[1], z]))), '%(face)s', '%(face)s', 0.6, 1, 99);
'''
SLICES='''
    const NS = 11;
    for (let j = 0; j < NS; j++) {
      const z = -1.6 * H + 3.2 * H * j / (NS - 1), o = pj([0, 0, z]);
      const dd = poly(OUT.map(a => pj([a[0], a[1], z]))) + poly(HOLE.map(a => pj([a[0], a[1], z])));
      push(mid, dd, j === (fs > 0 ? NS - 1 : 0) ? '%(face)s' : 'none', '%(face)s', 2.2, 0.28 + 0.6 * Math.abs(j / (NS - 1) - (fs > 0 ? 0 : 1)) * 0 + (j === (fs > 0 ? NS - 1 : 0) ? 0.72 : 0), o[2]);
    }
'''
WIRE='''
    [[OUT, CO], [HOLE, CH]].forEach(([lp, cs]) => {
      [H, -H].forEach(z => {
        const near = z * fs > 0;
        push(mid, poly(lp.map(a => pj([a[0], a[1], z]))), 'none', '%(face)s', near ? 2.4 : 1.4, near ? 0.95 : 0.4, near ? 50 : -50);
      });
      cs.forEach(i => { const a = lp[i], p = pj([a[0], a[1], H]), q = pj([a[0], a[1], -H]); push(mid, 'M' + P2(p) + 'L' + P2(q), 'none', '%(face)s', 1.4, 0.7, 0); });
    });
'''
ORBIT_RENDER='''
    const o = orb(t);
    (o.full || []).forEach(fu => {
      let run = '', side = null, n = 0, kz = 0;
      const flush = () => { if (run) push(side ? fore : back, run, 'none', fu.c, fu.w, fu.o * (side ? 1 : 0.6), side ? -1e3 : -1e3 + kz / Math.max(n, 1)); run = ''; n = 0; kz = 0; };
      fu.pts.forEach((p, j) => { const sd = front(p); if (sd !== side) { flush(); side = sd; } const q = pj(p); run += (run ? 'L' : 'M') + P2(q); n++; kz += q[2]; });
      flush();
    });
    o.trails.forEach(tr => {
      const n = tr.pts.length;
      for (let j = 1; j < n; j++) {
        const a = tr.pts[j - 1], b = tr.pts[j], pa = pj(a), pb = pj(b), mp = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2];
        push(front(mp) ? fore : back, 'M' + P2(pa) + 'L' + P2(pb), 'none', tr.c, tr.w * (0.15 + 0.85 * j / n) * (pa[3] + pb[3]) / 2, Math.pow(j / n, 1.6), (pa[2] + pb[2]) / 2);
      }
    });
    o.bodies.forEach(b => { const q = pj(b.p); push(front(b.p) ? fore : back, circ(q, b.r * K * q[3]), b.c, 'none', 0, 1, q[2] + 0.01); });
    const by = (a, b) => a.k - b.k;
    return { items: back.sort(by).concat(mid.sort(by), fore.sort(by)) };
'''
def plate(fname, title, num, sub, bg, fg, mode, colours, orbjs, yaw0=-0.35, amp=0.45):
    core=CORE%{'out':json.dumps(g['out']).replace(' ',''),'hole':json.dumps(g['hole']).replace(' ',''),'co':json.dumps(g['co']),'ch':json.dumps(g['ch']),'yaw0':yaw0,'amp':amp}
    body={'solid':SOLID,'slices':SLICES,'wire':WIRE}[mode]%colours
    js=f'''  renderVals() {{{core}{body}
{orbjs}{ORBIT_RENDER}  }}'''
    cap='#5c574e' if bg==PAPER else '#9a9386'
    return f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
<style>
body{{margin:0}}
</style>
</helmet>
<div style="width: 640px; height: 640px; background: {bg}; color: {fg}; font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif; display: flex; flex-direction: column">
<svg width="640" height="588" viewBox="0 0 640 588" role="img" aria-label="{title}: a 3D Principia P with simulated three-body orbits"><sc-for list="{{{{items}}}}" as="it" hint-placeholder-count="40"><path d="{{{{it.d}}}}" fill="{{{{it.f}}}}" stroke="{{{{it.s}}}}" stroke-width="{{{{it.w}}}}" opacity="{{{{it.o}}}}" fill-rule="evenodd" stroke-linecap="round" stroke-linejoin="round"></path></sc-for></svg>
<div style="display: flex; justify-content: space-between; align-items: baseline; padding: 0 28px; height: 52px; font-size: 13px; color: {cap}">
<span style="font-weight: 700; color: {fg}">{num}</span>
<span>{sub}</span>
</div>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"speed":{{"editor":"range","min":0,"max":3,"step":0.1,"default":1}},"$preview":{{"width":640,"height":640}}}}'>
class Component extends DCLogic {{
  componentDidMount() {{
    this.t0 = performance.now();
    const rm = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (rm) {{ this.setState({{ t: 3 }}); return; }}
    const loop = () => {{ this.setState({{ t: (performance.now() - this.t0) / 1000 }}); this.raf = requestAnimationFrame(loop); }};
    this.raf = requestAnimationFrame(loop);
  }}
  componentWillUnmount() {{ cancelAnimationFrame(this.raf); }}
{js}
}}
</script>
</body>
</html>
'''
HC=[0.0,0.36,0.0]
boards={}
# I. threaded eight
e=d['eight'][:-1,0,:]
orb=f'''    const E = {fl(e,4)}, N = E.length / 2, S = 1.42, th = 0.72, u = [Math.cos(th), 0, Math.sin(th)];
    const at = (j) => {{ const q = ((j % N) + N) % N, X = E[2*q] * S, Y = E[2*q+1] * S; return [{HC[0]} + X * u[0], {HC[1]} + Y, X * u[2]]; }};
    const orb = (t) => {{
      const k0 = Math.floor(((t / 5) % 1) * N), TL = Math.floor(N * 0.4), cols = ['{M}', '{Y}', '{C}'], full = [], trails = [], bodies = [];
      
      for (let i = 0; i < 3; i++) {{ const k = k0 + Math.floor(i * N / 3), pts = []; for (let j = k - TL; j <= k; j += 2) pts.push(at(j)); trails.push({{ pts, c: cols[i], w: 7 }}); bodies.push({{ p: at(k), c: cols[i], r: 0.095 }}); }}
      return {{ full, trails, bodies }};
    }};'''
boards['Threaded.dc.html']=plate('Threaded','Threaded eight','I','Figure-eight choreography, threaded through the P',PAPER,INK,'solid',{'s0':'#3a3631','s1':'#b3ab9b','face':INK},orb)
# II. slices + Lagrange ring
orb=f'''    const R = 1.22, th = 0.62, u = [Math.cos(th), 0, Math.sin(th)];
    const at = (a) => [{HC[0]} + R * Math.cos(a) * u[0], {HC[1]} + R * Math.sin(a), R * Math.cos(a) * u[2]];
    const orb = (t) => {{
      const cols = ['{M}', '{Y}', '{C}'], ring = [], trails = [], bodies = [], w = t * 0.7;
      for (let j = 0; j <= 96; j++) ring.push(at(j / 96 * 2 * Math.PI));
      const full = [{{ pts: ring, c: '{INK}', o: 0.2, w: 2 }}];
      const bp = [0, 1, 2].map(i => at(w + i * 2 * Math.PI / 3));
      for (let i = 0; i < 3; i++) {{ const tp = []; for (let j = 0; j <= 24; j++) tp.push(at(w + i * 2 * Math.PI / 3 - 0.9 + 0.9 * j / 24)); trails.push({{ pts: tp, c: cols[i], w: 8 }}); bodies.push({{ p: bp[i], c: cols[i], r: 0.14 }}); }}
      [0, 1, 2].forEach(i => {{ const a = bp[i], b = bp[(i + 1) % 3], ch = []; for (let j = 0; j <= 16; j++) ch.push([a[0] + (b[0] - a[0]) * j / 16, a[1] + (b[1] - a[1]) * j / 16, a[2] + (b[2] - a[2]) * j / 16]); full.push({{ pts: ch, c: '{INK}', o: 0.55, w: 1.6 }}); }});
      return {{ full, trails, bodies }};
    }};'''
boards['Slices.dc.html']=plate('Slices','Slices','II','Lagrange&#39;s triangle round a P made of slices',PAPER,INK,'slices',{'face':INK},orb,yaw0=-0.5,amp=0.55)
# III. Burrau ring, solid paper P on ink
b=d['burrau'][::2][:1281]
orb=f'''    const B = [{fl(b[:,0,:])}, {fl(b[:,1,:])}, {fl(b[:,2,:])}], N = B[0].length / 2, S = 0.62, Y0 = -0.7, ca = Math.cos(0.35), sa = Math.sin(0.35);
    const at = (A, j) => {{ const X = A[2*j] * S, Z = A[2*j+1] * S; return [X * ca + Z * sa, Y0, -X * sa + Z * ca]; }};
    const orb = (t) => {{
      const k = Math.min(N - 1, Math.floor(((t % 20.5) / 18) * (N - 1))), cols = ['{M}', '{Y}', '{C}'], rs = [0.075, 0.085, 0.095], full = [], trails = [], bodies = [];
      B.forEach((A, i) => {{
        
        const tp = []; for (let j = Math.max(0, k - 160); j <= k; j++) tp.push(at(A, j)); trails.push({{ pts: tp, c: cols[i], w: 5 }});
        bodies.push({{ p: at(A, k), c: cols[i], r: rs[i] }});
      }});
      return {{ full, trails, bodies }};
    }};'''
boards['BurrauRing.dc.html']=plate('Burrau ring','Burrau ring','III','Burrau&#39;s 3-4-5 problem, played round the foot of the P',INK,PAPER,'solid',{'s0':'#5b5549','s1':'#d9d2c2','face':PAPER},orb,yaw0=-0.25,amp=0.5)
# IV. wire butterfly
bt=d['butterfly'][:-1]
orb=f'''    const B = [{fl(bt[:,0,:],4)}, {fl(bt[:,1,:],4)}, {fl(bt[:,2,:],4)}], N = B[0].length / 2, S = 1.9, th = 0.8, u = [Math.cos(th), 0, Math.sin(th)];
    const at = (A, j) => {{ const q = ((j % N) + N) % N, X = A[2*q] * S, Y = A[2*q+1] * S * 1.6; return [{HC[0]} + X * u[0], {HC[1]} + Y, X * u[2]]; }};
    const orb = (t) => {{
      const k = Math.floor(((t / 8) % 1) * N), cols = ['{M}', '{Y}', '{C}'], full = [], trails = [], bodies = [];
      B.forEach((A, i) => {{
        
        const tp = []; for (let j = k - Math.floor(N * 0.3); j <= k; j += 2) tp.push(at(A, j)); trails.push({{ pts: tp, c: cols[i], w: 6 }});
        bodies.push({{ p: at(A, k), c: cols[i], r: 0.075 }});
      }});
      return {{ full, trails, bodies }};
    }};'''
boards['WireButterfly.dc.html']=plate('Wire butterfly','Wire butterfly','IV','Butterfly I through a wireframe P',INK,PAPER,'wire',{'face':PAPER},orb,yaw0=-0.6,amp=0.5)

mo=d['moth'][:-1]
def mothorb(S,th,period,frac,w,r):
    return f'''    const B = [{fl(mo[:,0,:],4)}, {fl(mo[:,1,:],4)}, {fl(mo[:,2,:],4)}], N = B[0].length / 2, S = {S}, th = {th}, u = [Math.cos(th), 0, Math.sin(th)];
    const at = (A, j) => {{ const q = ((j % N) + N) % N, X = A[2*q] * S, Y = A[2*q+1] * S; return [X * u[0], 0.05 + Y, X * u[2]]; }};
    const orb = (t) => {{
      const k = Math.floor(((t / {period}) % 1) * N), cols = ['{M}', '{Y}', '{C}'], trails = [], bodies = [];
      B.forEach((A, i) => {{
        const tp = []; for (let j = k - Math.floor(N * {frac}); j <= k; j++) tp.push(at(A, j)); trails.push({{ pts: tp, c: cols[i], w: {w} }});
        bodies.push({{ p: at(A, k), c: cols[i], r: {r} }});
      }});
      return {{ full: [], trails, bodies }};
    }};'''
boards['MothSolid.dc.html']=plate('Moth, solid','Moth','V','Moth I round a solid P, long trails fading out',PAPER,INK,'solid',{'s0':'#3a3631','s1':'#b3ab9b','face':INK},mothorb(1.3,0.75,15,0.3,9,0.1),yaw0=-0.4,amp=0.5)
boards['MothNight.dc.html']=plate('Moth, night','Moth at night','VI','The same orbit on dark, round a paper P',INK,PAPER,'solid',{'s0':'#5b5549','s1':'#d9d2c2','face':PAPER},mothorb(1.3,0.75,15,0.3,9,0.1),yaw0=-0.4,amp=0.5)
for k,v in boards.items(): open('p4/'+k,'w').write(v); print(k,len(v))
