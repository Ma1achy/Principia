"""One pass, every candidate field. Merges tb_ftle (Benettin + diffusion) with
event tracking (t_end, binary_id) so a probe costs one integration instead of two.
"""
import numpy as np
import tb, tb_ftle


def integrate_all(r0, v0, t_max, dt, eps=0.03, d0=1e-8, renorm_every=200,
                  sample_every=25, seed=0, shared_dir=True):
    eps2 = eps * eps
    r, v = r0.copy(), v0.copy()
    n = r.shape[0]

    rng = np.random.default_rng(seed)
    if shared_dir:
        d1 = rng.normal(size=(1,) + r.shape[1:]); d1 /= np.linalg.norm(d1)
        pert = np.repeat(d1, n, axis=0)
    else:
        pert = rng.normal(size=r.shape)
        pert /= np.linalg.norm(pert.reshape(n, -1), axis=1)[:, None, None]
    rs = r + d0 * pert; vs = v.copy()

    S = np.zeros(n); nre = np.zeros(n, dtype=np.int64)
    cnt = 0.0; St = 0.0; Stt = 0.0
    Sy = np.zeros(n); Sty = np.zeros(n)
    dmin = np.full(n, np.inf)
    t_end = np.full(n, np.nan)

    a = tb.accel(r, eps2); as_ = tb.accel(rs, eps2)
    steps = int(round(t_max / dt))
    MT = tb.M

    for s in range(steps):
        v += 0.5*dt*a;   r += dt*v;   a = tb.accel(r, eps2);   v += 0.5*dt*a
        vs += 0.5*dt*as_; rs += dt*vs; as_ = tb.accel(rs, eps2); vs += 0.5*dt*as_

        if s % renorm_every == 0 and s > 0:
            dr = rs - r; dv = vs - v
            d = np.sqrt(np.einsum('nki,nki->n', dr, dr) + np.einsum('nki,nki->n', dv, dv))
            ok = d > 1e-300
            S[ok] += np.log(d[ok] / d0); nre[ok] += 1
            sc = np.where(ok, d0 / np.maximum(d, 1e-300), 1.0)[:, None, None]
            rs = r + dr * sc; vs = v + dv * sc

        if s % sample_every == 0:
            t = (s + 1) * dt
            y = np.log(np.maximum(tb_ftle.inertia(r), 1e-300))
            cnt += 1.0; St += t; Stt += t*t; Sy += y; Sty += t*y
            pd = tb.pair_dists(r)
            dmin = np.minimum(dmin, pd.min(axis=1))
            tight = np.argmin(pd, axis=1)
            third = np.array([2, 1, 0])[tight]
            live = np.isnan(t_end)
            if live.any():
                for b in range(3):
                    sel = live & (third == b)
                    if not sel.any():
                        continue
                    o = [k for k in range(3) if k != b]
                    mb = MT[o].sum()
                    rc = (MT[o[0]]*r[sel][:, o[0], :] + MT[o[1]]*r[sel][:, o[1], :]) / mb
                    vc = (MT[o[0]]*v[sel][:, o[0], :] + MT[o[1]]*v[sel][:, o[1], :]) / mb
                    dr2 = r[sel][:, b, :] - rc; dv2 = v[sel][:, b, :] - vc
                    dist = np.sqrt(np.einsum('ij,ij->i', dr2, dr2))
                    spec = 0.5*np.einsum('ij,ij->i', dv2, dv2) - mb/np.maximum(dist, 1e-9)
                    esc = (spec > 0) & (np.einsum('ij,ij->i', dr2, dv2) > 0)
                    t_end[np.nonzero(sel)[0][esc]] = t

    T = steps * dt
    ftle = np.where(nre > 0, S / max(T, 1e-12), np.nan)
    den = cnt*Stt - St*St
    diffusion = np.where(abs(den) > 1e-12, (cnt*Sty - St*Sy) / den, np.nan)
    censored = np.isnan(t_end)
    return dict(r=r, v=v, ftle=ftle, diffusion=diffusion, dmin=dmin,
                t_end=np.where(censored, T, t_end), censored=censored,
                binary_id=np.argmin(tb.pair_dists(r), axis=1), T=T)
