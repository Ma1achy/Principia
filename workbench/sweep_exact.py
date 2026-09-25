"""Same 8-region x 3-playhead x 3-jitter matrix as sweep.py, but eps=0 (gauge intact)."""
import numpy as np, tb, tb_all_exact as XA, refine_test as R
from sweep import REGIONS

def probe(cx, cy, t, jf, body=0, N=4, ens=7, eta=0.01, n_sync=64):
    r0, v0, gid, _, _ = tb.burrau_grid(N, N, cx, cy, 0.05, body=body, ens=ens,
                                       jitter_frac=jf, seed=0)
    res = XA.integrate_all_exact(r0, v0, t_max=t, eta=eta, n_sync=n_sync)
    n = R.shape_vec(res['r'])
    cls = tb.classify(dict(r=res['r'], v=res['v']))
    joint = (cls < 3).astype(int) * 3 + res['binary_id']
    KE = 0.5*np.einsum('k,nki->n', tb.M, res['v']*res['v'])
    E = tb.energy(res['r'], res['v'], 0.0)
    g = lambda f: float(np.nanmean([f(np.nonzero(gid == k)[0]) for k in range(N*N)]))
    return dict(
        shape = g(lambda s: np.mean(np.linalg.norm(n[s]-n[s].mean(0), axis=1))),
        event = g(lambda s: R.disagree(joint[s])),
        t_end = g(lambda s: np.std(res['t_end'][s])) / max(res['T'], 1e-9),
        ftle  = g(lambda s: np.nanstd(res['ftle'][s])),
        diff  = g(lambda s: np.nanstd(res['diffusion'][s])),
        dmin  = g(lambda s: np.std(res['dmin'][s])),
        KE    = g(lambda s: np.std(KE[s])),
        E     = g(lambda s: np.std(E[s])),
        _drift = float(np.median(res['drift'])),
        _floored = float(res['floored'].mean()),
    )

FIELDS = ['shape','event','t_end','ftle','diff','dmin','KE','E']
