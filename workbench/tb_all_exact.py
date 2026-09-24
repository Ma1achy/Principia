"""
Unsoftened combined integrator: every candidate field, no softening, gauge intact.

Architecture mirrors production lockstep. The run is divided into K equal SYNC INTERVALS.
Inside an interval each trajectory advances with its own scale-covariant adaptive dt; at each
sync boundary every trajectory is at the same playhead, and that is where we
  - sample the diffusion regression,
  - update d_min,
  - test for escape (t_end) and record the tightest pair (event class),
  - renormalise the Benettin shadow.

The shadow is carried in the same batch as its parent but is stepped with the PARENT's dt, so
the pair stays synchronised; taking dt from the shadow's own configuration would desynchronise
them and corrupt the separation measurement.
"""
import numpy as np
import tb, tb_exact, tb_ftle


def integrate_all_exact(r0, v0, t_max, eta=0.01, n_sync=64, d0=1e-8,
                        seed=0, shared_dir=True, floor_frac=1e-8, max_iter=200000):
    n = r0.shape[0]
    r = r0.copy(); v = v0.copy()

    rng = np.random.default_rng(seed)
    if shared_dir:
        d1 = rng.normal(size=(1,) + r.shape[1:]); d1 /= np.linalg.norm(d1)
        pert = np.repeat(d1, n, axis=0)
    else:
        pert = rng.normal(size=r.shape)
        pert /= np.linalg.norm(pert.reshape(n, -1), axis=1)[:, None, None]
    # Seed the shadow at a DIMENSIONLESS separation d0. Using an absolute offset makes the
    # first interval contribute -log(R), i.e. -log(alpha) per rescaling, which shows up as a
    # constant additive FTLE offset of log(alpha)/T (verified: 0.0866 = log(2)/8).
    R_init = np.sqrt(np.maximum(tb_ftle.inertia(r), 1e-300) / tb.M.sum())
    rs = r + (d0 * R_init)[:, None, None] * pert; vs = v.copy()

    E0 = tb.energy(r, v, 0.0)
    S = np.zeros(n); nre = np.zeros(n, dtype=np.int64)
    cnt = 0.0; St = 0.0; Stt = 0.0
    Sy = np.zeros(n); Sty = np.zeros(n)
    dmin = np.full(n, np.inf)
    t_end = np.full(n, np.nan)
    floored = np.zeros(n, dtype=bool)
    dt_floor = floor_frac * t_max
    total_iter = 0

    a = tb_exact.accel_exact(r); as_ = tb_exact.accel_exact(rs)

    for k in range(n_sync):
        t_target = (k + 1) * t_max / n_sync
        t = np.full(n, k * t_max / n_sync)
        it = 0
        while it < max_iter:
            act = t < t_target - 1e-15
            if not act.any():
                break
            dt = tb_exact._dt_crit(r, eta)          # from the MAIN trajectory only
            hit = dt < dt_floor
            floored |= (hit & act)
            dt = np.minimum(np.maximum(dt, dt_floor), t_target - t)
            dt = np.where(act, dt, 0.0)[:, None, None]

            v += 0.5*dt*a;   r += dt*v;   a = tb_exact.accel_exact(r);   v += 0.5*dt*a
            vs += 0.5*dt*as_; rs += dt*vs; as_ = tb_exact.accel_exact(rs); vs += 0.5*dt*as_
            t += dt[:, 0, 0]
            it += 1
        total_iter += it

        # ---- sync point: every trajectory is at the same playhead ----
        # DIMENSIONLESS phase-space separation. |dr|^2 + |dv|^2 is NOT scale-covariant:
        # under r -> a r, t -> a^{3/2} t we have dr ~ a but dv ~ a^{-1/2}, so the naive sum
        # mixes two different scalings and makes FTLE gauge-dependent (measured: 25% swing).
        # Normalising by the instantaneous hyperradius R and its velocity scale sqrt(GM/R)
        # makes both terms dimensionless, restoring the gauge.
        dr = rs - r; dv = vs - v
        Rg = np.sqrt(np.maximum(tb_ftle.inertia(r), 1e-300) / tb.M.sum())
        d = np.sqrt(np.einsum('nki,nki->n', dr, dr) / Rg**2
                    + np.einsum('nki,nki->n', dv, dv) * Rg / tb.M.sum())
        ok = d > 1e-300
        S[ok] += np.log(d[ok] / d0); nre[ok] += 1
        sc = np.where(ok, d0 / np.maximum(d, 1e-300), 1.0)[:, None, None]
        rs = r + dr*sc; vs = v + dv*sc
        as_ = tb_exact.accel_exact(rs)

        y = np.log(np.maximum(tb_ftle.inertia(r), 1e-300))
        cnt += 1.0; St += t_target; Stt += t_target**2; Sy += y; Sty += t_target*y

        pd = tb.pair_dists(r)
        dmin = np.minimum(dmin, pd.min(axis=1))
        third = np.array([2, 1, 0])[np.argmin(pd, axis=1)]
        live = np.isnan(t_end)
        if live.any():
            for b in range(3):
                sel = live & (third == b)
                if not sel.any():
                    continue
                o = [q for q in range(3) if q != b]
                mb = tb.M[o].sum()
                rc = (tb.M[o[0]]*r[sel][:, o[0], :] + tb.M[o[1]]*r[sel][:, o[1], :]) / mb
                vc = (tb.M[o[0]]*v[sel][:, o[0], :] + tb.M[o[1]]*v[sel][:, o[1], :]) / mb
                dr2 = r[sel][:, b, :] - rc; dv2 = v[sel][:, b, :] - vc
                dist = np.sqrt(np.einsum('ij,ij->i', dr2, dr2))
                spec = 0.5*np.einsum('ij,ij->i', dv2, dv2) - mb/np.maximum(dist, 1e-12)
                esc = (spec > 0) & (np.einsum('ij,ij->i', dr2, dv2) > 0)
                t_end[np.nonzero(sel)[0][esc]] = t_target

    ftle = np.where(nre > 0, S / max(t_max, 1e-12), np.nan)
    den = cnt*Stt - St*St
    diffusion = np.where(abs(den) > 1e-12, (cnt*Sty - St*Sy) / den, np.nan)
    censored = np.isnan(t_end)
    drift = np.abs((tb.energy(r, v, 0.0) - E0) / np.maximum(np.abs(E0), 1e-30))
    return dict(r=r, v=v, ftle=ftle, diffusion=diffusion, dmin=dmin,
                t_end=np.where(censored, t_max, t_end), censored=censored,
                binary_id=np.argmin(tb.pair_dists(r), axis=1), T=t_max,
                drift=drift, floored=floored, iters=total_iter)
