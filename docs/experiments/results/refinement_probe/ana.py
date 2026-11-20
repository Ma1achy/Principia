import numpy as np, tb, json

NX = NY = 96
HALF = 3.0
z = np.load('/home/claude/ck_wide.npz')
r, v = z['r'], z['v']
hx = (2 * HALF) / (NX - 1)

cls = tb.classify(dict(r=r, v=v))
C = cls.reshape(NY, NX)
W = z['word'].reshape(NY, NX, -1)
L = z['wlen'].reshape(NY, NX)

print("class fractions (esc0, esc1, esc2, still-bound):",
      [f"{np.mean(cls==k):.3f}" for k in range(4)])
print(f"grid {NX}x{NY}  h={hx:.5f}  box half-width {HALF}\n")


def f_out(C, s):
    return 0.5 * ((C[:, :-s] != C[:, s:]).mean() + (C[:-s, :] != C[s:, :]).mean())


def wdiff(Wa, Wb, La, Lb):
    n = np.minimum(La, Lb)
    idx = np.arange(Wa.shape[-1])
    mask = idx[None, None, :] < n[..., None]
    return ((Wa != Wb) & mask).any(axis=-1) | (La != Lb)


def f_word(W, L, s):
    x = wdiff(W[:, :-s], W[:, s:], L[:, :-s], L[:, s:]).mean()
    y = wdiff(W[:-s], W[s:], L[:-s], L[s:]).mean()
    return 0.5 * (x + y)


print("=== TEST 1: does uncertain fraction shrink with scale? ===")
print(" s    eps        f_outcome   f_word")
scales = [1, 2, 4, 8, 16]
fo, fw = [], []
for s in scales:
    a, b = f_out(C, s), f_word(W, L, s)
    fo.append(a); fw.append(b)
    print(f" {s:<4} {s*hx:.5f}    {a:.4f}      {b:.4f}")
e = np.array(scales) * hx
ao = np.polyfit(np.log(e), np.log(np.maximum(fo, 1e-9)), 1)[0]
aw = np.polyfit(np.log(e), np.log(np.maximum(fw, 1e-9)), 1)[0]
print(f"\n  GLOBAL alpha (outcome) = {ao:.3f}")
print(f"  GLOBAL alpha (word)    = {aw:.3f}")
print("  (alpha ~1 => smooth boundary, refining localises it;"
      " alpha ~0 => riddled, refining futile)\n")

print("=== TEST 1b: LOCAL alpha per block — does it discriminate? ===")
for BS in [12, 16, 24]:
    nb = NY // BS
    am, fm = np.full((nb, nb), np.nan), np.zeros((nb, nb))
    for by in range(nb):
        for bx in range(nb):
            blk = C[by*BS:(by+1)*BS, bx*BS:(bx+1)*BS]
            fs = []
            for s in [1, 2, 4]:
                a = (blk[:, :-s] != blk[:, s:]).mean()
                b = (blk[:-s, :] != blk[s:, :]).mean()
                fs.append(0.5*(a+b))
            fs = np.array(fs); fm[by, bx] = fs[0]
            if (fs > 0).all():
                am[by, bx] = np.polyfit(np.log(np.array([1., 2., 4.])*hx), np.log(fs), 1)[0]
    ok = ~np.isnan(am)
    qs = np.nanpercentile(am, [5, 25, 50, 75, 95])
    print(f" block {BS:>2}x{BS:<2} ({nb}x{nb} blocks, {ok.sum()} valid): "
          f"alpha p5/25/50/75/95 = {' '.join(f'{q:5.2f}' for q in qs)}  std={np.nanstd(am):.3f}")
    print(f"            local f(h): min={fm.min():.3f} med={np.median(fm):.3f} max={fm.max():.3f}"
          f"   frac blocks with f(h)>0.5: {(fm>0.5).mean():.2f}")

print("\n=== TEST 2: does S_word fire where the outcome does not? ===")
s = 1
od = np.concatenate([(C[:, :-s] != C[:, s:]).ravel(), (C[:-s, :] != C[s:, :]).ravel()])
wd = np.concatenate([wdiff(W[:, :-s], W[:, s:], L[:, :-s], L[:, s:]).ravel(),
                     wdiff(W[:-s], W[s:], L[:-s], L[s:]).ravel()])
print(f"  outcome differs      {od.mean():.4f}")
print(f"  word differs         {wd.mean():.4f}")
print(f"  both                 {(od & wd).mean():.4f}")
print(f"  WORD ONLY (outcome agrees)  {(wd & ~od).mean():.4f}")
print(f"  OUTCOME ONLY (words agree)  {(od & ~wd).mean():.4f}")
if wd.mean() > 0:
    print(f"  precision of word as a split trigger = {(od & wd).sum()/wd.sum():.3f}")
if od.mean() > 0:
    print(f"  recall                               = {(od & wd).sum()/od.sum():.3f}")
print(f"  word length: median={np.median(L)} max={L.max()} frac saturated(>=24)={(L>=24).mean():.3f}")

json.dump(dict(alpha_out=float(ao), alpha_word=float(aw),
               f_out=[float(x) for x in fo], f_word=[float(x) for x in fw],
               scales=[float(x) for x in e]),
          open('/home/claude/ana.json', 'w'), indent=1)
