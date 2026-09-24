import numpy as np, tb, tb_all, refine_test as R

REGIONS = [('near-field',1.0,3.0,0), ('deep interior',0.0,0.0,0), ('mid-field',1.0,6.0,0),
           ('far r~10',1.0,13.0,0), ('body2 core',1.0,-1.0,2), ('body2 mid',1.0,-5.0,2),
           ('body1 slice',-2.0,-1.0,1), ('body1 far',-2.0,-7.0,1)]

def probe(cx, cy, t, jf, body=0, N=5, ens=7):
    r0, v0, gid, _, _ = tb.burrau_grid(N, N, cx, cy, 0.05, body=body, ens=ens,
                                       jitter_frac=jf, seed=0)
    res = tb_all.integrate_all(r0, v0, t_max=t, dt=5e-4, eps=0.03)
    n = R.shape_vec(res['r'])
    cls = tb.classify(dict(r=res['r'], v=res['v']))
    joint = (cls < 3).astype(int) * 3 + res['binary_id']
    E = tb.energy(res['r'], res['v'], 0.03**2)
    KE = 0.5*np.einsum('k,nki->n', tb.M, res['v']*res['v'])
    def g(f): return float(np.nanmean([f(np.nonzero(gid == k)[0]) for k in range(N*N)]))
    return dict(
        shape   = g(lambda s: np.mean(np.linalg.norm(n[s]-n[s].mean(0), axis=1))),
        event   = g(lambda s: R.disagree(joint[s])),
        t_end   = g(lambda s: np.std(res['t_end'][s])) / max(res['T'], 1e-9),
        ftle    = g(lambda s: np.nanstd(res['ftle'][s])),
        diff    = g(lambda s: np.nanstd(res['diffusion'][s])),
        dmin    = g(lambda s: np.std(res['dmin'][s])),
        KE      = g(lambda s: np.std(KE[s])),
        E       = g(lambda s: np.std(E[s])),
    )

FIELDS = ['shape','event','t_end','ftle','diff','dmin','KE','E']
