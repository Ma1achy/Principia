import numpy as np, tb

def integrate_events(r0, v0, t_max, dt, eps=0.03, sample_every=25):
    """Adds: t_end (first escape detection, censored at t_max) and binary_id
    (which pair is currently tightest -- an EVENT CLASS defined at every t, not just at termination)."""
    eps2 = eps*eps
    r, v = r0.copy(), v0.copy()
    n = r.shape[0]
    t_end = np.full(n, np.nan)
    a = tb.accel(r, eps2)
    steps = int(round(t_max/dt))
    for s in range(steps):
        v += 0.5*dt*a; r += dt*v; a = tb.accel(r, eps2); v += 0.5*dt*a
        if s % sample_every == 0:
            pd = tb.pair_dists(r)
            tight = np.argmin(pd, axis=1)
            third = np.array([2,1,0])[tight]
            live = np.isnan(t_end)
            if live.any():
                for b in range(3):
                    sel = live & (third == b)
                    if not sel.any(): continue
                    others = [k for k in range(3) if k != b]
                    mb = tb.M[others].sum()
                    rc = (tb.M[others[0]]*r[sel][:,others[0],:] + tb.M[others[1]]*r[sel][:,others[1],:]) / mb
                    vc = (tb.M[others[0]]*v[sel][:,others[0],:] + tb.M[others[1]]*v[sel][:,others[1],:]) / mb
                    dr = r[sel][:,b,:] - rc; dv = v[sel][:,b,:] - vc
                    dist = np.sqrt(np.einsum('ij,ij->i', dr, dr))
                    sp = 0.5*np.einsum('ij,ij->i', dv, dv) - mb/np.maximum(dist,1e-9)
                    esc = (sp > 0) & (np.einsum('ij,ij->i', dr, dv) > 0)
                    idx = np.nonzero(sel)[0][esc]
                    t_end[idx] = (s+1)*dt
    binary_id = np.argmin(tb.pair_dists(r), axis=1)
    censored = np.isnan(t_end)
    t_end = np.where(censored, t_max, t_end)
    return dict(r=r, v=v, t_end=t_end, binary_id=binary_id, censored=censored)
