"""
Unsoftened (eps = 0) three-body integration with per-trajectory adaptive timesteps.

WHY: softening puts a FIXED length eps into the force law, which does not transform under
dynamical similarity (r -> a r, t -> a^{3/2} t). That breaks the scale gauge Principia
quotients out, and contaminates every scale-quotiented measurement (findings doc §7.5).

The fix is not a smaller eps -- it is no eps at all. Close encounters are then resolved by
shrinking dt instead of by softening the force. The timestep criterion

    dt = eta * min_ij ( r_ij^{3/2} / sqrt(G (m_i + m_j)) )

is the local two-body free-fall time, so it is SCALE-COVARIANT: under r -> a r it scales as
a^{3/2}, exactly as t does. No fixed length or time enters anywhere, so the gauge survives.

Each trajectory carries its own dt and its own clock; the batch is stepped with masking until
every trajectory reaches t_max. Variable-timestep leapfrog is not strictly symplectic, so
energy drift is monitored rather than assumed.
"""
import numpy as np
import tb

PAIRS = [(0, 1), (0, 2), (1, 2)]


def accel_exact(r):
    """Newtonian acceleration, no softening. r: (S,3,2) -> (S,3,2)"""
    a = np.zeros_like(r)
    for i, j in PAIRS:
        d = r[:, j, :] - r[:, i, :]
        d2 = np.einsum('sk,sk->s', d, d)
        inv = np.where(d2 > 0, d2 ** -1.5, 0.0)
        f = d * inv[:, None]
        a[:, i, :] += tb.M[j] * f
        a[:, j, :] -= tb.M[i] * f
    return a


def _dt_crit(r, eta):
    """Scale-covariant timestep: eta * local two-body free-fall time of the tightest pair."""
    best = None
    for i, j in PAIRS:
        d = r[:, j, :] - r[:, i, :]
        rij = np.sqrt(np.einsum('sk,sk->s', d, d))
        tau = rij ** 1.5 / np.sqrt(tb.M[i] + tb.M[j])
        best = tau if best is None else np.minimum(best, tau)
    return eta * best


def integrate_exact(r0, v0, t_max, eta=0.02, max_iter=400000, floor_frac=1e-7):
    """KDK leapfrog, no softening, per-trajectory adaptive dt. Returns fields + diagnostics.

    floor_frac bounds dt from below RELATIVE to t_max (so it too carries no absolute scale);
    trajectories that hit it are flagged rather than silently trusted.
    """
    r = r0.copy(); v = v0.copy()
    n = r.shape[0]
    t = np.zeros(n)
    E0 = tb.energy(r, v, 0.0)
    dmin = np.full(n, np.inf)
    floored = np.zeros(n, dtype=bool)
    dt_floor = floor_frac * t_max

    a = accel_exact(r)
    it = 0
    while it < max_iter:
        act = t < t_max
        if not act.any():
            break
        dt = _dt_crit(r, eta)
        hit = dt < dt_floor
        floored |= (hit & act)
        dt = np.maximum(dt, dt_floor)
        dt = np.minimum(dt, t_max - t)
        dt = np.where(act, dt, 0.0)[:, None, None]

        v += 0.5 * dt * a
        r += dt * v
        a = accel_exact(r)
        v += 0.5 * dt * a
        t += dt[:, 0, 0]

        dmin = np.minimum(dmin, tb.pair_dists(r).min(axis=1))
        it += 1

    E1 = tb.energy(r, v, 0.0)
    drift = np.abs((E1 - E0) / np.maximum(np.abs(E0), 1e-30))
    return dict(r=r, v=v, drift=drift, dmin=dmin, iters=it,
                floored=floored, t=t, complete=bool((t >= t_max - 1e-12).all()))
