src=open('plates.py').read()
exec(src[:src.index('SOLID=')])   # CORE, fl, data, colours
cl=json.load(open('clear.json'))
RAMP=[('#ff7ab3','#c40f3f'),('#ffd84a','#ee6a0a'),('#62e6ff','#1052d6')]
def core(spin,K=172,CX=320,CY=306):
    c=CORE%{'out':json.dumps(g['out']).replace(' ',''),'hole':json.dumps(g['hole']).replace(' ',''),'co':json.dumps(g['co']),'ch':json.dumps(g['ch']),'yaw0':spin,'amp':0}
    c=c.replace('const H = 0.2, D = 7, K = 150, CX = 320, CY = 292, PITCH = 0.3','const H = 0.2, D = 4.2, K = %d * (this.props.zoom ?? 1), CX = %d, CY = %d,' % (K,CX,CY) + ' PITCH = 0.17, WT = this.props.weight ?? 1')
    c=c.replace("const push = (grp, d, f, s, w, o, k) => grp.push({ d, f, s, w: (+w).toFixed(2), o: (+o).toFixed(3), k });","const push = (grp, d, f, s, w, o, k, bm) => grp.push({ d, f, s, w: (+w).toFixed(2), o: (+o).toFixed(3), k, bm: bm || 'normal' });")
    c=c.replace("const t = ((this.state && this.state.t) || 0) * (this.props.speed ?? 1);","const t = this.props.still ? (this.props.frame ?? 17) : ((this.state && this.state.t) || 0) * (this.props.speed ?? 1);")
    return c
COMMON='''
    const RAMP = %(ramp)s, NIGHT = %(night)s, BM = NIGHT ? 'screen' : 'multiply', glowB = [], glowF = [];
    const o = orb(t), bodies = o.bodies.map(b => b.p);
    const rgb = (h) => hx(h), css = (c) => 'rgb(' + c.map(x => Math.max(0, Math.min(255, Math.round(x)))).join(',') + ')';
    const lightAt = (p, n, base, gain) => { let c = base.slice(); o.bodies.forEach(bd => { const b = bd.p, dv = [b[0]-p[0], b[1]-p[1], b[2]-p[2]], dl = Math.hypot(...dv), nd = Math.max(0, (n[0]*dv[0] + n[1]*dv[1] + n[2]*dv[2]) / dl), lc = rgb(RAMP[bd.i][0]); c = c.map((x, j) => x + lc[j] * gain * nd / (1 + 1.2 * dl * dl)); }); return css(c); };
'''
PSOLID='''
    const S0 = hx('%(s0)s'), S1 = hx('%(s1)s');
    [OUT, HOLE].forEach(lp => {
      for (let i = 0; i < lp.length; i++) {
        const a = lp[i], b = lp[(i + 1) %% lp.length];
        let nx = b[1] - a[1], ny = -(b[0] - a[0]); const nl = Math.hypot(nx, ny); nx /= nl; ny /= nl;
        const nv = rot([nx, ny, 0]); if (nv[2] <= 0.01) continue;
        const lam = Math.max(0, nv[0] * LL[0] + nv[1] * LL[1] + nv[2] * LL[2]), u2 = 0.12 + 0.88 * lam;
        const base = S0.map((x, j) => x + (S1[j] - x) * u2);
        const q = [pj([a[0], a[1], H]), pj([b[0], b[1], H]), pj([b[0], b[1], -H]), pj([a[0], a[1], -H])];
        const col = lightAt([(a[0]+b[0])/2, (a[1]+b[1])/2, 0], [nx, ny, 0], base, NIGHT ? 1.6 : 1.4);
        push(mid, poly(q), col, col, 0.8, 1, (q[0][2] + q[1][2] + q[2][2] + q[3][2]) / 4);
      }
    });
    const zf = fs * H, faceD = poly(OUT.map(a => pj([a[0], a[1], zf]))) + poly(HOLE.map(a => pj([a[0], a[1], zf])));
    push(mid, faceD, '%(face)s', '%(face)s', 0.6, 1, 99);
    const pools = o.bodies.map(bd => { const b = bd.p, inF = (b[2] - zf) * fs > 0, q = pj([b[0], b[1], zf]), dz = Math.abs(b[2] - zf), r = (0.95 + 1.1 * dz) * K * q[3];
      return { d: circ(q, r), f: 'url(#h' + bd.i + '-%(vid)s)', o: Math.min(1, (NIGHT ? 1.5 : 1.3) * (inF ? 1 : 0.12) / (1 + 0.8 * dz * dz)).toFixed(3) }; });
'''
PSLICES='''
    const NS = 11;
    for (let j = 0; j < NS; j++) {
      const z = -1.6 * H + 3.2 * H * j / (NS - 1), oo = pj([0, 0, z]), last = j === (fs > 0 ? NS - 1 : 0);
      const dd = poly(OUT.map(a => pj([a[0], a[1], z]))) + poly(HOLE.map(a => pj([a[0], a[1], z])));
      const col = lightAt([0, 0.2, z], [0, 0, fs], hx('%(face)s'), 0.9);
      push(mid, dd, last ? '%(face)s' : 'none', col, 2.2, last ? 1 : 0.55, oo[2]);
    }
    const faceD = 'M0,0', pools = [];
'''
PWIRE='''
    [[OUT, CO], [HOLE, CH]].forEach(([lp, cs]) => {
      [H, -H].forEach(z => { const near = z * fs > 0; push(mid, poly(lp.map(a => pj([a[0], a[1], z]))), 'none', lightAt([0, 0.2, z], [0, 0, z > 0 ? 1 : -1], hx('%(face)s'), 1.2), near ? 2.4 : 1.4, near ? 0.95 : 0.4, near ? 50 : -50); });
      cs.forEach(i => { const a = lp[i], p = pj([a[0], a[1], H]), q = pj([a[0], a[1], -H]); push(mid, 'M' + P2(p) + 'L' + P2(q), 'none', '%(face)s', 1.4, 0.7, 0); });
    });
    const faceD = 'M0,0', pools = [];
'''
RENDER='''
    const PBp = OUT.map(a => pj([a[0], a[1], 0])), PB = [Math.min(...PBp.map(q => q[0])), Math.max(...PBp.map(q => q[0])), Math.min(...PBp.map(q => q[1])), Math.max(...PBp.map(q => q[1]))];
    (o.extra || []).forEach(fu => {
      let run = '', side = null;
      const flush = () => { if (run) push(side ? fore : back, run, 'none', fu.c, fu.w, fu.o * (side ? 1 : 0.6), -1e3); run = ''; };
      fu.pts.forEach(p => { const sd = front(p); if (sd !== side) { flush(); side = sd; } run += (run ? 'L' : 'M') + P2(pj(p)); });
      flush();
    });
    // trails: one continuous tapered ribbon per body, cut into abutting chunks so nothing overlaps
    o.trails.forEach(tr => {
      // insert an exact point wherever the path crosses the P's mid-plane, and never let a piece straddle it
      const raw = tr.pts, pts = [], cuts = [0];
      for (let j = 0; j < raw.length; j++) {
        pts.push(raw[j]);
        if (j < raw.length - 1 && front(raw[j]) !== front(raw[j + 1])) {
          const a = raw[j], b = raw[j + 1], s = a[2] / (a[2] - b[2]);
          pts.push([a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s, 0]); cuts.push(pts.length - 1);
        }
      }
      const n = pts.length; if (n < 4) return;
      const Pp = pts.map(pj), C0 = RAMP[tr.i][1], C1 = RAMP[tr.i][0];
      // arc length on screen, so taper, fade and chunking follow the path rather than the sample count
      const Lc = [0]; for (let j = 1; j < n; j++) Lc.push(Lc[j - 1] + Math.hypot(Pp[j][0] - Pp[j - 1][0], Pp[j][1] - Pp[j - 1][1]));
      const Lt = Lc[n - 1] || 1;
      const CHN = 72; { let j = 0; for (let c = 1; c < CHN; c++) { const target = c * Lt / CHN; while (j < n - 1 && Lc[j] < target) j++; cuts.push(j); } }
      cuts.push(n - 1);
      const bnd = [...new Set(cuts)].sort((a, b) => a - b);
      const nr = Pp.map((p, j) => { const a = Pp[Math.max(0, j - 2)], b = Pp[Math.min(n - 1, j + 2)]; const dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy) || 1; return [-dy / l, dx / l]; });
      // width: taper by length, then cap by the local radius of curvature so tight turns never fold the ribbon
      let Wd = Pp.map((p, j) => %(tw)s * WT * p[3] * Math.pow(Lc[j] / Lt, 0.75));
      Wd = Wd.map((w, j) => { if (j === 0 || j === n - 1) return w; const a = Pp[j - 1], b = Pp[j], c2 = Pp[j + 1];
        const A = Math.hypot(b[0] - a[0], b[1] - a[1]), Bq = Math.hypot(c2[0] - b[0], c2[1] - b[1]), Cq = Math.hypot(c2[0] - a[0], c2[1] - a[1]);
        const ar = Math.abs((b[0] - a[0]) * (c2[1] - a[1]) - (b[1] - a[1]) * (c2[0] - a[0])) / 2; const rc = ar > 1e-6 ? A * Bq * Cq / (4 * ar) : 1e9; return Math.min(w, 1.7 * rc); });
      Wd = Wd.map((w, j) => { let m = w; for (let q = Math.max(0, j - 3); q <= Math.min(n - 1, j + 3); q++) m = Math.min(m, Wd[q] + Math.abs(q - j) * 0.6); return m; });
      const rib = (j0, j1, sc) => { let L = '', R = []; for (let j = j0; j <= j1; j++) { const p = Pp[j], h = Wd[j] * sc / 2; L += (j === j0 ? 'M' : 'L') + (p[0] + nr[j][0] * h).toFixed(2) + ',' + (p[1] + nr[j][1] * h).toFixed(2); R.push((p[0] - nr[j][0] * h).toFixed(2) + ',' + (p[1] - nr[j][1] * h).toFixed(2)); } return L + 'L' + R.reverse().join('L') + 'Z'; };
      for (let c = 0; c < bnd.length - 1; c++) {
        const j0 = bnd[c], j1 = bnd[c + 1]; if (j1 <= j0) continue;
        let zs = 0; for (let j = j0; j <= j1; j++) zs += pts[j][2];
        const jm = Math.round((j0 + j1) / 2), u = Lc[jm] / Lt, side = zs * fs > 0, grp = side ? fore : back, gg = side ? glowF : glowB, kk = Pp[jm][2];
        const depth = Math.min(1, 0.45 + 0.55 * (Pp[jm][3] - 0.7) / 0.6);
        const fade = Math.pow(u, 1.25) * depth, fadeCore = Math.pow(u, 2.4) * depth;
        const col = mix(C0, C1, Math.pow(u, 1.1)), bright = mix(C1, '#ffffff', 0.25), hot = mix(C1, '#ffffff', 0.75);
        // only glow that actually overlaps the P on screen goes under it; everything else stays in one layer, so no seam at the P's plane
        const pm = Pp[jm], overP = pm[0] > PB[0] - 30 && pm[0] < PB[1] + 30 && pm[1] > PB[2] - 30 && pm[1] < PB[3] + 30;
        const gl = side ? glowF : glowB;
        // soft neon: a thin hot core and a translucent tube stay crisp, the rest of the colour lives in the bloom
        push(grp, rib(j0, j1, 0.8), col, 'none', 0, fade * 0.5, kk, BM);
        push(grp, rib(j0, j1, 0.3), NIGHT ? hot : bright, 'none', 0, fadeCore, kk + 0.0005, NIGHT ? 'screen' : BM);
        gl.push({ d: rib(j0, j1, 3.2), f: col, o: (fade * 0.55).toFixed(3), bm: BM },
                { d: rib(j0, j1, 1.4), f: bright, o: (fade * 0.85).toFixed(3), bm: BM },
                { d: rib(j0, j1, 0.55), f: hot, o: (fadeCore * 0.9).toFixed(3), bm: NIGHT ? 'screen' : 'normal' });
      }
    });
    const GLW = this.props.glow ?? 1.3, orbGlow = [];
    o.bodies.forEach(bd => { const q = pj(bd.p), r = bd.r * K * q[3], grp = front(bd.p) ? fore : back;
      // an emissive ball of light: white-hot core, saturated colour, soft falloff
      push(grp, circ(q, r * 3.4 * GLW), 'url(#e' + bd.i + '-%(vid)s)', 'none', 0, 1, 500 + q[2], NIGHT ? 'screen' : 'normal');
      orbGlow.push([front(bd.p), { d: circ(q, r * 1.6 * GLW), f: RAMP[bd.i][0], o: '1', bm: BM }, { d: circ(q, r * 0.75), f: '#ffffff', o: '1', bm: NIGHT ? 'screen' : 'normal' }]); });
    orbGlow.forEach(([f2, a, b]) => (f2 ? glowF : glowB).push(a, b));
    const by = (a, b) => a.k - b.k;
    const TR = this.props.transparent ?? true, BD = this.props.backdrop ?? (NIGHT ? '#0d0c0b' : 'transparent');
    return { bgCss: TR ? BD : '%(v1)s', bgFill: TR ? 'none' : 'url(#vg-%(vid)s)', size: this.props.size ?? 640, itemsBack: back.sort(by), itemsMid: mid.sort(by), itemsB: fore.sort(by), pools, faceD, glowB, glowF };
'''
def board(title,vid,night,spin,pmode,pcol,orbjs,tw=9,frame=(172,320,306)):
    js='  renderVals() {'+core(spin,*frame)+orbjs+COMMON%{'ramp':json.dumps(RAMP),'night':str(night).lower()}+{'solid':PSOLID,'slices':PSLICES,'wire':PWIRE}[pmode]%dict(pcol,vid=vid)+RENDER%{'vid':vid,'tw':tw,'v1':('#0a0908' if night else '#d6ccb8')}+'\n  }'
    grads=''.join(f'<radialGradient id="b{i}-{vid}" cx="0.36" cy="0.32" r="0.75"><stop offset="0" stop-color="#ffffff"></stop><stop offset="0.28" stop-color="{a}"></stop><stop offset="1" stop-color="{b}"></stop></radialGradient><radialGradient id="e{i}-{vid}" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="#ffffff"></stop><stop offset="0.13" stop-color="#ffffff" stop-opacity="0.95"></stop><stop offset="0.26" stop-color="{a}"></stop><stop offset="0.42" stop-color="{a}" stop-opacity="0.6"></stop><stop offset="0.66" stop-color="{b}" stop-opacity="0.2"></stop><stop offset="1" stop-color="{b}" stop-opacity="0"></stop></radialGradient><radialGradient id="h{i}-{vid}" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="{a}" stop-opacity="0.8"></stop><stop offset="0.4" stop-color="{b}" stop-opacity="0.3"></stop><stop offset="1" stop-color="{b}" stop-opacity="0"></stop></radialGradient>' for i,(a,b) in enumerate(RAMP))
    if night:
        filt=f'<filter id="bl-{vid}" x="-40%" y="-40%" width="180%" height="180%" color-interpolation-filters="sRGB"><feGaussianBlur in="SourceGraphic" stdDeviation="2" result="b1"></feGaussianBlur><feComponentTransfer in="b1" result="b1b"><feFuncA type="linear" slope="1.4"></feFuncA></feComponentTransfer><feGaussianBlur in="SourceGraphic" stdDeviation="7" result="b2"></feGaussianBlur><feComponentTransfer in="b2" result="b2b"><feFuncA type="linear" slope="2.2"></feFuncA></feComponentTransfer><feGaussianBlur in="SourceGraphic" stdDeviation="18" result="b3"></feGaussianBlur><feComponentTransfer in="b3" result="b3b"><feFuncA type="linear" slope="1.8"></feFuncA></feComponentTransfer><feGaussianBlur in="SourceGraphic" stdDeviation="42" result="b4"></feGaussianBlur><feColorMatrix in="b4" type="matrix" values="1.3 0.4 0.15 0 0.05  0.1 0.38 0.05 0 0  0 0.04 0.18 0 0  0 0 0 1.1 0" result="hal"></feColorMatrix><feMerge><feMergeNode in="hal"></feMergeNode><feMergeNode in="b3b"></feMergeNode><feMergeNode in="b2b"></feMergeNode><feMergeNode in="b1b"></feMergeNode></feMerge></filter>'
        gl=f'<g filter="url(#bl-{vid})" style="mix-blend-mode: screen">'
        v0,v1,bgc=('#2d2925','#0a0908','#1b1917')
    else:
        filt=f'<filter id="bl-{vid}" x="-40%" y="-40%" width="180%" height="180%" color-interpolation-filters="sRGB"><feGaussianBlur in="SourceGraphic" stdDeviation="2.5" result="b1"></feGaussianBlur><feGaussianBlur in="SourceGraphic" stdDeviation="9" result="b2"></feGaussianBlur><feComponentTransfer in="b2" result="b2"><feFuncA type="linear" slope="1.6"></feFuncA></feComponentTransfer><feGaussianBlur in="SourceGraphic" stdDeviation="24" result="b3"></feGaussianBlur><feColorMatrix in="b3" type="matrix" values="1.15 0.25 0.1 0 0.02  0.08 0.6 0.05 0 0  0 0.05 0.4 0 0  0 0 0 0.8 0" result="hal"></feColorMatrix><feMerge><feMergeNode in="hal"></feMergeNode><feMergeNode in="b2"></feMergeNode><feMergeNode in="b1"></feMergeNode></feMerge></filter>'
        gl=f'<g filter="url(#bl-{vid})" opacity="0.8" style="mix-blend-mode: multiply">'
        v0,v1,bgc=('#f6f1e6','#d6ccb8',PAPER)
    item='<path d="{{it.d}}" fill="{{it.f}}" stroke="{{it.s}}" stroke-width="{{it.w}}" opacity="{{it.o}}" style="mix-blend-mode: {{it.bm}}" stroke-linecap="round" stroke-linejoin="round"></path>'
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
<div style="width: {{{{size}}}}px; height: {{{{size}}}}px; background: {{{{bgCss}}}}; line-height: 0">
<svg width="{{{{size}}}}" height="{{{{size}}}}" viewBox="0 0 640 640" role="img" aria-label="{title}: a 3D Principia P with three simulated bodies"><defs><radialGradient id="vg-{vid}" cx="50%" cy="44%" r="72%"><stop offset="0" stop-color="{v0}"></stop><stop offset="0.6" stop-color="{bgc}"></stop><stop offset="1" stop-color="{v1}"></stop></radialGradient>{grads}{filt}<mask id="mP-{vid}" maskUnits="userSpaceOnUse" x="-2000" y="-2000" width="6000" height="6000"><rect x="-2000" y="-2000" width="6000" height="6000" fill="#ffffff"></rect><sc-for list="{{{{itemsMid}}}}" as="mk" hint-placeholder-count="20"><path d="{{{{mk.d}}}}" fill="#000000" stroke="#000000" stroke-width="1.2"></path></sc-for></mask><clipPath id="pc-{vid}"><path d="{{{{faceD}}}}"></path></clipPath></defs><rect width="640" height="640" fill="{{{{bgFill}}}}"></rect><sc-for list="{{{{itemsBack}}}}" as="it" hint-placeholder-count="20">{item}</sc-for><sc-for list="{{{{itemsMid}}}}" as="it" hint-placeholder-count="30">{item}</sc-for><g clip-path="url(#pc-{vid})" style="mix-blend-mode: screen"><sc-for list="{{{{pools}}}}" as="pl" hint-placeholder-count="3"><path d="{{{{pl.d}}}}" fill="{{{{pl.f}}}}" opacity="{{{{pl.o}}}}"></path></sc-for></g><sc-for list="{{{{itemsB}}}}" as="it" hint-placeholder-count="20">{item}</sc-for>{gl}<g mask="url(#mP-{vid})"><sc-for list="{{{{glowB}}}}" as="gw" hint-placeholder-count="10"><path d="{{{{gw.d}}}}" fill="{{{{gw.f}}}}" opacity="{{{{gw.o}}}}" style="mix-blend-mode: {{{{gw.bm}}}}"></path></sc-for></g><sc-for list="{{{{glowF}}}}" as="gw" hint-placeholder-count="20"><path d="{{{{gw.d}}}}" fill="{{{{gw.f}}}}" opacity="{{{{gw.o}}}}" style="mix-blend-mode: {{{{gw.bm}}}}"></path></sc-for></g></svg>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"speed":{{"editor":"range","min":0,"max":3,"step":0.1,"default":1}},"still":{{"editor":"boolean","default":false}},"frame":{{"editor":"range","min":0,"max":30,"step":0.1,"default":17}},"zoom":{{"editor":"range","min":0.6,"max":1.8,"step":0.05,"default":1}},"weight":{{"editor":"range","min":0.5,"max":3,"step":0.1,"default":1}},"size":{{"editor":"int","min":16,"max":1024,"default":640}},"transparent":{{"editor":"boolean","default":true}},"backdrop":{{"editor":"color","default":"{'#0d0c0b' if night else 'transparent'}","options":["#0d0c0b","#1b1917","#efe9dc","transparent"]}},"glow":{{"editor":"range","min":0.5,"max":2.5,"step":0.1,"default":1.3}},"$preview":{{"width":640,"height":640}}}}'>
class Component extends DCLogic {{
  componentDidMount() {{
    this.t0 = performance.now();
    if (this.props.still) return;
    const rm = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (rm) {{ this.setState({{ t: 6 }}); return; }}
    const loop = () => {{ this.setState({{ t: 6 + (performance.now() - this.t0) / 1000 }}); this.raf = requestAnimationFrame(loop); }};
    this.raf = requestAnimationFrame(loop);
  }}
  componentWillUnmount() {{ cancelAnimationFrame(this.raf); }}
{js}
}}
</script>
</body>
</html>
'''
def planar(key, period, frac, p, r, yscale=1.0):
    _,_,th,x0,y0,S,*rest=p; yk=rest[0] if (rest and key!='moth') else 1.0
    arr=d[key][:-1]
    return f'''
    const B = [{fl(arr[:,0,:],4)}, {fl(arr[:,1,:],4)}, {fl(arr[:,2,:],4)}], N = B[0].length / 2, S = {S}, YK = {yk}, th = {th}, u = [Math.cos(th), 0, Math.sin(th)];
    const at = (A, j) => {{ const q = ((j % N) + N) % N, X = A[2*q] * S, Y = A[2*q+1] * S * YK; return [{x0} + X * u[0], {y0} + Y, X * u[2]]; }};
    const orb = (t) => {{ const k = Math.floor(((t / {period}) % 1) * N), TL = Math.floor(N * {frac});
      return {{ trails: B.map((A, i) => {{ const pts = []; for (let j = k - TL; j <= k; j++) pts.push(at(A, j)); return {{ pts, i }}; }}), bodies: B.map((A, i) => ({{ p: at(A, k), i, r: {r} }})) }}; }};
'''

def freeorb(key, period, frac, P, r, src=None):
    import numpy as np
    psi,phi,cx,cy,S,tau=P
    dv=np.array([np.cos(phi),np.sin(phi),0.0]); e=np.array([0,0,1.0]); k=dv
    e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
    arr=(d if src is None else src)[key][:-1]; arr=arr[::max(1,len(arr)//900)]; c,s=np.cos(psi),np.sin(psi)
    rot=np.stack([arr[...,0]*c-arr[...,1]*s, arr[...,0]*s+arr[...,1]*c],-1)*S
    return f'''
    const B = [{fl(rot[:,0,:],4)}, {fl(rot[:,1,:],4)}, {fl(rot[:,2,:],4)}], N = B[0].length / 2, DV = {fl(dv,5)}, EV = {fl(e,5)};
    const at = (A, j) => {{ const q = ((j % N) + N) % N, X = A[2*q], Y = A[2*q+1]; return [{cx} + X * DV[0] + Y * EV[0], {cy} + X * DV[1] + Y * EV[1], X * DV[2] + Y * EV[2]]; }};
    const orb = (t) => {{ const k = Math.floor(((t / {period}) % 1) * N), TL = Math.floor(N * {frac});
      return {{ trails: B.map((A, i) => {{ const pts = []; for (let j = k - TL; j <= k; j++) pts.push(at(A, j)); return {{ pts, i }}; }}), bodies: B.map((A, i) => ({{ p: at(A, k), i, r: {r} }})) }}; }};
'''
MF=json.load(open('moth_free.json'))
# The plan is unclear here; moth_frame.json and frames.json are not committed, so the defaults below are used (they gave the committed boards).
MFR=json.load(open('moth_frame.json')) if __import__('os').path.exists('moth_frame.json') else [172,320,306]
E=d['eight'][:-1,0,:]
_,_,th,x0,y0,S,yk=cl['eight']
eight=f'''
    const E = {fl(E,4)}, N = E.length / 2, th = {th}, u = [Math.cos(th), 0, Math.sin(th)];
    const at = (j) => {{ const q = ((j % N) + N) % N, X = E[2*q] * {S}, Y = E[2*q+1] * {S} * {yk}; return [{x0} + X * u[0], {y0} + Y, X * u[2]]; }};
    const orb = (t) => {{ const k0 = Math.floor(((t / 7) % 1) * N), TL = Math.floor(N * 0.3), trails = [], bodies = [];
      for (let i = 0; i < 3; i++) {{ const k = k0 + Math.floor(i * N / 3), pts = []; for (let j = k - TL; j <= k; j++) pts.push(at(j)); trails.push({{ pts, i }}); bodies.push({{ p: at(k), i, r: 0.085 }}); }}
      return {{ trails, bodies }}; }};
'''
_,_,th,x0,y0,R=cl['lagrange']
lag=f'''
    const th = {th}, u = [Math.cos(th), 0, Math.sin(th)];
    const at = (a) => [{x0} + {R} * Math.cos(a) * u[0], {y0} + {R} * Math.sin(a), {R} * Math.cos(a) * u[2]];
    const orb = (t) => {{ const w = t * 0.45, ring = [], trails = [], bodies = [];
      for (let j = 0; j <= 160; j++) ring.push(at(j / 160 * 2 * Math.PI));
      for (let i = 0; i < 3; i++) {{ const a0 = w + i * 2 * Math.PI / 3, pts = []; for (let j = 0; j <= 120; j++) pts.push(at(a0 - 1.3 + 1.3 * j / 120)); trails.push({{ pts, i }}); bodies.push({{ p: at(a0), i, r: 0.12 }}); }}
      return {{ trails, bodies, extra: [{{ pts: ring, c: '{INK}', o: 0.16, w: 1.6 }}] }}; }};
'''
_,_,y0,S,tilt=cl['burrau']
bb=d['burrau'][:2561]
bur=f'''
    const B = [{fl(bb[:,0,:])}, {fl(bb[:,1,:])}, {fl(bb[:,2,:])}], N = B[0].length / 2, ca = Math.cos(0.35), sa = Math.sin(0.35);
    const at = (A, j) => {{ const X = A[2*j] * {S}, Z = A[2*j+1] * {S}; return [X * ca + Z * sa, {y0}, -X * sa + Z * ca]; }};
    const orb = (t) => {{ const k = Math.min(N - 1, Math.floor(((t % 21) / 18) * (N - 1))), rs = [0.075, 0.085, 0.095];
      return {{ trails: B.map((A, i) => {{ const pts = []; for (let j = Math.max(0, k - 320); j <= k; j++) pts.push(at(A, j)); return {{ pts, i }}; }}), bodies: B.map((A, i) => ({{ p: at(A, k), i, r: rs[i] }})) }}; }};
'''
cp=lambda night:{'s0':'#2e2a25','s1':'#d6cebd','face':'#c2b9a6'} if night else {'s0':'#141210','s1':'#7d7466','face':INK}
mspin=-0.62
B={
 'MothSolid.dc.html':board('Moth','v',False,mspin,'solid',cp(False),planar('moth',24,0.34,cl['moth'],0.085),frame=(134,326,325)),
 'MothNight.dc.html':board('Moth at night','vi',True,mspin,'solid',cp(True),planar('moth',24,0.34,cl['moth'],0.085),frame=(134,326,325)),
 'Threaded.dc.html':board('Threaded eight','i',False,0.55,'solid',cp(False),eight,tw=8),
 'Slices.dc.html':board('Slices','ii',False,0.45,'slices',{'face':INK},lag,tw=10),
 'BurrauRing.dc.html':board('Burrau ring','iii',True,-0.4,'solid',cp(True),bur,tw=6),
 'WireButterfly.dc.html':board('Wire butterfly','iv',True,0.45,'wire',{'face':PAPER},planar('butterfly',9,0.3,cl['butterfly'],0.07),tw=7),
}

CAT=np.load('catalog.npz'); FR=json.load(open('frames.json')) if __import__('os').path.exists('frames.json') else {}
for nm,title,vid,T in [('butterfly1','Butterfly I','b1',6.2356),('butterfly2','Butterfly II','b2',7.0039),('goggles','Goggles','gg',10.4668),('dragonfly','Dragonfly','df',21.271)]:
    P=json.load(open(f'orb_{nm}.json'))['found'][0][2]
    fr=tuple(FR.get(nm,[172,320,306]))
    B['Orb_'+nm+'.dc.html']=board(title,vid,False,mspin,'solid',cp(False),freeorb(nm,round(T*1.7,1),0.3 if T<15 else 0.2,P,0.085,src=CAT),tw=8,frame=fr)

def adaptorb(period, frac, r):
    AD=json.load(open('dragon_adapt.json'))
    pt=', '.join('{ t: %s, p: %s }' % (json.dumps(b['t']).replace(' ',''), json.dumps(b['p']).replace(' ','')) for b in AD)
    return f'''
    const AD = [{pt}], PER = {period}, TLF = {frac};
    const seek = (b, u) => {{ const T = AD[b].t; let lo = 0, hi = T.length - 1; if (u < T[0]) return -1; while (lo < hi) {{ const m = (lo + hi + 1) >> 1; if (T[m] <= u) lo = m; else hi = m - 1; }} return lo; }};
    const P3 = (b, i) => {{ const A = AD[b].p, n = AD[b].t.length, q = ((i % n) + n) % n; return [A[3*q], A[3*q+1], A[3*q+2]]; }};
    const at = (b, u) => {{ const T = AD[b].t, n = T.length; u = ((u % 1) + 1) % 1; let i = seek(b, u); const ta = i < 0 ? T[n-1] - 1 : T[i], tb = i + 1 < n ? T[i + 1] : T[0] + 1, a = P3(b, i), c = P3(b, i + 1), s = (u - ta) / ((tb - ta) || 1); return [a[0] + (c[0] - a[0]) * s, a[1] + (c[1] - a[1]) * s, a[2] + (c[2] - a[2]) * s]; }};
    const orb = (t) => {{
      const u = ((t / PER) % 1 + 1) % 1, u0 = u - TLF;
      return {{ trails: [0, 1, 2].map(b => {{
        const T = AD[b].t, pts = [at(b, u0)];
        if (u0 < 0) {{ for (let i = 0; i < T.length; i++) if (T[i] > u0 + 1) pts.push(P3(b, i)); for (let i = 0; i < T.length && T[i] < u; i++) pts.push(P3(b, i)); }}
        else {{ for (let i = 0; i < T.length && T[i] < u; i++) if (T[i] > u0) pts.push(P3(b, i)); }}
        pts.push(at(b, u)); return {{ pts, i: b }}; }}), bodies: [0, 1, 2].map(b => ({{ p: at(b, u), i: b, r: {r} }})) }};
    }};
'''
DFR=json.load(open('dragon_frame.json'))   # required: a missing file must fail, not fall back to a different framing
B['Dragonfly.dc.html']=board('Dragonfly','dfa',False,mspin,'solid',cp(False),adaptorb(36,0.2,0.085),tw=8,frame=tuple(DFR))
B['DragonflyNight.dc.html']=board('Dragonfly at night','dfb',True,mspin,'solid',cp(True),adaptorb(36,0.2,0.085),tw=8,frame=tuple(DFR))
for k,v in B.items(): open('p4/'+k,'w').write(v); print(k,len(v))
