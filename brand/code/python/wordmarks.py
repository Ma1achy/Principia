src=open('final.py').read()
exec(src[:src.index("for k,v in B.items():")])     # everything the icon boards use
import json
L=json.load(open('word_loops.json')); PG=json.load(open('pgeo.json'))
hp=[{'pts':PG['out'],'hole':False},{'pts':PG['hole'],'hole':True}]   # high-poly P
rest=L[2:]
from matplotlib.path import Path as _P
def _xc(q):
    pts=np.array(q['pts'])
    if not q['hole']: return float((pts[:,0].min()+pts[:,0].max())/2)
    for o in rest:
        if not o['hole'] and _P(np.array(o['pts'])).contains_point(pts.mean(0)):
            op=np.array(o['pts']); return float((op[:,0].min()+op[:,0].max())/2)
    return float(pts[:,0].mean())
SUBL=json.load(open('sub_loops.json'))
def loops_js(Hp,Hr,sub=False):
    items=[{'p':[round(v,4) for xy in q['pts'] for v in xy],'H':Hp,'xc':0} for q in hp]+[{'p':[round(v,4) for xy in q['pts'] for v in xy],'H':Hr,'xc':round(_xc(q),4)} for q in rest]
    if sub: items+= [{'p':[round(v,4) for xy in q['pts'] for v in xy],'H':0,'xc':0,'c':1} for q in SUBL]
    xs=sorted(set(round(it['xc'],1) for it in items))
    for it in items: it['r']=xs.index(round(it['xc'],1))
    return json.dumps(items).replace(' ','')
PWORD='''
    const LP = %(loops)s, OVER = %(over)s;
    const S0 = hx('%(s0)s'), S1 = hx('%(s1)s'), faces = {}, PERL = %(perl)s;
    const pjL = (p, xc) => { if (!PERL || !xc) return pj(p); const v = rot([p[0] - xc, p[1], p[2]]), f = D / (D - v[2]); return [CX + K * (xc + v[0] * f), CY - K * v[1] * f, v[2], f]; };
    const wpos = (p, xc) => (!PERL || !xc) ? p : [p[0] - xc + xc * Math.cos(spin), p[1], p[2] + xc * Math.sin(spin)];
    LP.forEach((lp, li) => {
      if (OVER && li >= 2) return;
      const n = lp.p.length / 2, P = (i) => [lp.p[2 * (i %% n)], lp.p[2 * (i %% n) + 1]], H2 = lp.H;
      if (H2 > 0) for (let i = 0; i < n; i++) {
        const a = P(i), b = P(i + 1);
        let nx = b[1] - a[1], ny = -(b[0] - a[0]); const nl = Math.hypot(nx, ny) || 1; nx /= nl; ny /= nl;
        const nv = rot([nx, ny, 0]); if (nv[2] <= 0.01) continue;
        const lam = Math.max(0, nv[0] * LL[0] + nv[1] * LL[1] + nv[2] * LL[2]), u2 = 0.12 + 0.88 * lam;
        const base = S0.map((x, j) => x + (S1[j] - x) * u2);
        const q = [pjL([a[0], a[1], H2], lp.xc), pjL([b[0], b[1], H2], lp.xc), pjL([b[0], b[1], -H2], lp.xc), pjL([a[0], a[1], -H2], lp.xc)];
        const col = lightAt(wpos([(a[0]+b[0])/2, (a[1]+b[1])/2, 0], lp.xc), [nx, ny, 0], base, NIGHT ? 1.6 : 1.4);
        push(mid, poly(q), col, col, 0.8, 1, PERL ? lp.r * 1000 + (q[0][2] + q[1][2] + q[2][2] + q[3][2]) / 4 : (q[0][2] + q[1][2] + q[2][2] + q[3][2]) / 4);
      }
      const zf = fs * H2, key = lp.c ? 'S' : (PERL ? 'L' + lp.r : zf.toFixed(3)), pts = []; for (let i = 0; i < n; i++) { const a = P(i); pts.push(pjL([a[0], a[1], zf], lp.xc)); }
      faces[key] = (faces[key] || '') + poly(pts);
    });
    let faceD = '';
    Object.keys(faces).forEach(k => { if (k === 'S') { push(mid, faces[k], '%(subc)s', '%(subc)s', 0.4, 1, 97); return; } push(mid, faces[k], '%(face)s', '%(face)s', 0.6, 1, PERL ? (+k.slice(1)) * 1000 + 999 : 99); faceD += faces[k]; });
    if (OVER) {
      // flat type beside the 3D P: same cap height and baseline as the P on screen
      const top = pj([0, 1, fs * 0.2]), bot = pj([0, -1, fs * 0.2]), rgt = pj([0.76, -1, fs * 0.2]), KT = (bot[1] - top[1]) / 2;
      const OX = rgt[0] - KT * 0.76 + KT * 0.06, OY = bot[1] - KT;
      let td = '', tds = ''; LP.slice(2).forEach(lp => { const n = lp.p.length / 2, pts = []; for (let i = 0; i < n; i++) pts.push([OX + KT * lp.p[2 * i], OY - KT * lp.p[2 * i + 1]]); if (lp.c) tds += poly(pts); else td += poly(pts); });
      push(mid, td, '%(face)s', '%(face)s', 0.6, 1, 98); faceD += td; if (tds) push(mid, tds, '%(subc)s', '%(subc)s', 0.4, 1, 97);
    }
    const pools = o.bodies.map(bd => { const b = bd.p, q = pj([b[0], b[1], 0]), dz = Math.abs(b[2]), r = (0.95 + 1.1 * dz) * K * q[3];
      return { d: circ(q, r), f: 'url(#h' + bd.i + '-%(vid)s)', o: Math.min(1, 1.3 * ((b[2] * fs) > 0 ? 1 : 0.12) / (1 + 0.8 * dz * dz)).toFixed(3) }; });
'''
def wordboard(title,vid,spin,pitch,D,Hp,Hr,over,frame,W,Hh,perl=False,night=False,sub=False):
    pc={'s0':'#2e2a25','s1':'#d6cebd','face':'#efe9dc'} if night else {'s0':'#141210','s1':'#7d7466','face':INK}
    pblock=PWORD%dict(pc,perl=str(perl).lower(),loops=loops_js(Hp,Hr,sub),over=str(over).lower(),vid=vid,subc=('#c8c1b2' if night else '#3d3a34'))
    page=board(title,vid,night,spin,'solid',pc,adaptorb(36,0.2,0.085),tw=8,frame=frame)
    # swap the P-only geometry block for the word block
    a=page.index("    const S0 = hx('%s'), S1 = hx('%s');" % (pc['s0'],pc['s1'])); b=page.index("    const PBp = OUT.map")
    page=page[:a]+pblock+page[b:]
    page=page.replace('D = 4.2,','D = %s,' % D).replace('PITCH = 0.17,','PITCH = %s,' % pitch)
    page=page.replace('viewBox="0 0 640 640"','viewBox="0 0 %d %d"' % (W,Hh)).replace('<rect width="640" height="640"','<rect width="%d" height="%d"' % (W,Hh))
    page=page.replace('width: {{size}}px; height: {{size}}px;','width: %dpx; height: %dpx;' % (W,Hh)).replace('width="{{size}}" height="{{size}}"','width="%d" height="%d"' % (W,Hh))
    page=page.replace('"$preview":{"width":640,"height":640}','"$preview":{"width":%d,"height":%d}' % (W,Hh))
    return page
WF=json.load(open('word_frames.json'))   # required: a missing file must fail, not fall back to a different framing
def fr(k,dflt): return tuple(WF.get(k,dflt))
SPECS=[
 ('WordFlat',         'Principia, 2D',                'wa',  0.0,0.0,1e4,0.0,0.0,False,False,False,600),
 ('WordFlatNight',    'Principia, 2D, dark',          'wan', 0.0,0.0,1e4,0.0,0.0,False,False,True, 600),
 ('WordMixed',        'Principia, 3D P',              'wb', -0.62,0.17,4.2,0.2,0.0,True, False,False,600),
 ('Word3D',           'Principia, 3D',                'wc', -0.62,0.17,24, 0.2,0.2,False,False,False,600),
 ('WordFlatSub',      'Principia, 2D, subtitle',      'was', 0.0,0.0,1e4,0.0,0.0,False,True, False,360),
 ('WordFlatSubNight', 'Principia, 2D, subtitle, dark','wasn',0.0,0.0,1e4,0.0,0.0,False,True, True, 360),
 ('WordMixedSub',     'Principia, 3D P, subtitle',    'wbs',-0.62,0.17,4.2,0.2,0.0,True, True, False,420),
 ('Word3DSub',        'Principia, 3D, subtitle',      'wcs',-0.62,0.17,24, 0.2,0.2,False,True, False,480),
]
out={}
for key,title,vid,spin,pitch,D,Hp,Hr,over,sub,night,H in SPECS:
    base=key.replace('Night','') if key.startswith('WordFlat') and not sub else key
    out[key+'.dc.html']=wordboard(title,vid,spin,pitch,D,Hp,Hr,over,fr(key if key in WF else base,[60,200,H//2]),1600,H,night=night,sub=sub)

for k,v in out.items(): open('p4/'+k,'w').write(v); print(k,len(v))
