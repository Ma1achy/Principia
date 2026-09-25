"""
Does softening break the scale gauge, and does it contaminate the measurements?

Newtonian dynamical similarity: r -> a*r, t -> a^{3/2} t leaves the dynamics invariant.
The shape-sphere metric is scale-quotiented, so a rescaled system MUST give identical
shape-sphere spreads -- under exact Newtonian gravity.

Softened gravity has  (|r|^2 + eps^2)^{-3/2} : eps is a FIXED length, so it does not
transform. Prediction:
  * eps scaled with a  (eps -> a*eps)  -> gauge restored -> results identical
  * eps held fixed                     -> gauge broken   -> results differ
The size of that difference is the contamination in every measurement in the findings doc.
"""
import numpy as np, tb, refine_test as R


def scaled_probe(cx, cy, half, t, alpha, eps, scale_eps, N=5, ens=7, jf=0.5, body=0):
    r0, v0, gid, _, _ = tb.burrau_grid(N, N, cx, cy, half, body=body,
                                       ens=ens, jitter_frac=jf, seed=0)
    # dynamical similarity: lengths x a, times x a^{3/2}, velocities x a^{-1/2}
    r0 = r0 * alpha
    v0 = v0 * alpha ** -0.5
    t_s = t * alpha ** 1.5
    dt_s = 5e-4 * alpha ** 1.5
    e = eps * alpha if scale_eps else eps
    res = tb.integrate(r0, v0, t_max=t_s, dt=dt_s, eps=e)
    n = R.shape_vec(res['r'])                      # scale-quotiented by construction
    w = float(np.mean([np.mean(np.linalg.norm(n[np.nonzero(gid == g)[0]]
                     - n[np.nonzero(gid == g)[0]].mean(0), axis=1)) for g in range(N * N)]))
    b = float(np.mean(np.linalg.norm(n[np.arange(0, len(gid), 1 + ens)]
              - n[np.arange(0, len(gid), 1 + ens)].mean(0), axis=1)))
    return w, b, float(np.median(res['drift']))
