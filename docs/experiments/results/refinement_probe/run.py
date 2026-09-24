"""Resumable chunked integration: python3 run.py <n_chunks>"""
import numpy as np, os, sys, time, tb

NX = NY = 96
HALF = 3.0
CX, CY = 1.0, 3.0
DT = 2e-4
EPS = 0.03
CHUNK_STEPS = 40000          # 8.0 time units per chunk
TOTAL_CHUNKS = 5             # -> t_max = 40
CK = '/home/claude/ck_wide.npz'

r0, v0, gid, shape, hx = tb.burrau_grid(NX, NY, CX, CY, HALF)

if os.path.exists(CK):
    z = np.load(CK)
    r, v, dmin, word, wlen, prev, done = (z['r'], z['v'], z['dmin'], z['word'],
                                          z['wlen'], z['prev'], int(z['done']))
    E0 = z['E0']
else:
    r, v = r0.copy(), v0.copy()
    E0 = tb.energy(r, v, EPS * EPS)
    dmin = np.full(r.shape[0], np.inf)
    word = np.zeros((r.shape[0], 24), dtype=np.int8)
    wlen = np.zeros(r.shape[0], dtype=np.int32)
    prev = np.argmin(tb.pair_dists(r), axis=1).astype(np.int8)
    done = 0

n_run = int(sys.argv[1]) if len(sys.argv) > 1 else 1
eps2 = EPS * EPS
t0 = time.time()

for c in range(n_run):
    if done >= TOTAL_CHUNKS:
        break
    a = tb.accel(r, eps2)
    for s in range(CHUNK_STEPS):
        v += 0.5 * DT * a
        r += DT * v
        a = tb.accel(r, eps2)
        v += 0.5 * DT * a
        if s % 25 == 0:
            pd = tb.pair_dists(r)
            dmin = np.minimum(dmin, pd.min(axis=1))
            tight = np.argmin(pd, axis=1).astype(np.int8)
            ch = (tight != prev) & (wlen < 24)
            if ch.any():
                i = np.nonzero(ch)[0]
                word[i, wlen[i]] = tight[i] + 1
                wlen[i] += 1
            prev = tight
    done += 1
    np.savez(CK, r=r, v=v, dmin=dmin, word=word, wlen=wlen, prev=prev,
             done=done, E0=E0)
    E1 = tb.energy(r, v, eps2)
    dr = np.median(np.abs((E1 - E0) / np.abs(E0)))
    print(f"chunk {done}/{TOTAL_CHUNKS}  t={done*CHUNK_STEPS*DT:.1f}  "
          f"drift={dr:.2e}  {time.time()-t0:.0f}s", flush=True)

print("DONE" if done >= TOTAL_CHUNKS else f"paused at {done}/{TOTAL_CHUNKS}")
