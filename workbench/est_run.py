"""Measure spreads at 5 perturbation scales per region, so every estimator can be scored
against the SAME data. Energy is the ground-truth field (true exponent = 1.0)."""
import numpy as np, tb, tb_all_az as AA, refine_test as R

JFS = np.array([0.125, 0.25, 0.5, 1.0, 2.0])

def spreads_at(cx, cy, bd, half, jf, t=13.0, N=4, ens=7, eta=0.01, n_sync=32):
    r0, v0, gid, _, _ = tb.burrau_grid(N, N, cx, cy, half, body=bd, ens=ens,
                                       jitter_frac=jf, seed=0)
    res = AA.integrate_all_az(r0, v0, t_max=t, n_sync=n_sync, eta=eta)
    n = R.shape_vec(res['r'])
    KE = 0.5*np.einsum('k,nki->n', tb.M, res['v']*res['v'])
    E = tb.energy(res['r'], res['v'], 0.0)
    g = lambda f: float(np.nanmean([f(np.nonzero(gid == k)[0]) for k in range(N*N)]))
    return dict(
        E     = g(lambda s: np.std(E[s])),
        shape = g(lambda s: np.mean(np.linalg.norm(n[s]-n[s].mean(0), axis=1))),
        KE    = g(lambda s: np.std(KE[s])),
        diff  = g(lambda s: np.nanstd(res['diffusion'][s])),
    )

def scan(cx, cy, bd, half=0.05, **kw):
    return {jf: spreads_at(cx, cy, bd, half, jf, **kw) for jf in JFS}
