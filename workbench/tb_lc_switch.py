"""
Levi-Civita with PAIR SWITCHING.

Single-pair LC regularises only the pair it was built for; a close approach of either other
pair still enters through the unregularised perturbation term and destroys the integration
(measured: near-field drift 5.0e-04 unregularised -> 1.7e+01 with LC on the wrong pair).

In a chaotic triple every pair has close encounters at different times, so no static choice
works. This module switches: the run is divided into sync intervals; at each boundary the
state is held in Cartesian form, the currently-closest pair is identified per trajectory, and
the batch is regrouped so each subgroup integrates in the LC coordinates of its own pair.

This is the poor-man's version of global (Aarseth-Zare) regularisation: it handles one
regularised pair at a time and switches, rather than regularising two pairs simultaneously.
It fails only if two pairs are simultaneously close -- a genuine triple encounter.
"""
import numpy as np
import tb, tb_lc

PAIRS = [(0, 1), (0, 2), (1, 2)]


def integrate_lc_switch(r0, v0, t_max, n_sync=64, eta=0.01, max_steps=20000, M=None):
    Mv = tb.M if M is None else M
    n = r0.shape[0]
    r = r0.copy(); v = v0.copy()
    t = np.zeros(n)
    dmin = np.full(n, np.inf)
    switches = np.zeros(n, dtype=np.int64)
    prev_pair = np.full(n, -1)

    E0 = tb.energy(r, v, 0.0)

    for kk in range(n_sync):
        t_target = (kk + 1) * t_max / n_sync

        pd = tb.pair_dists(r)                       # (n,3) in PAIRS order
        which = np.argmin(pd, axis=1)
        switches += (prev_pair >= 0) & (which != prev_pair)
        prev_pair = which.copy()

        for pi, pair in enumerate(PAIRS):
            sel = np.nonzero(which == pi)[0]
            if len(sel) == 0:
                continue
            need = t[sel] < t_target - 1e-15
            if not need.any():
                continue
            sub = sel[need]
            dt_left = t_target - t[sub]

            sysm = tb_lc.LCSystem(pair[0], pair[1],
                                  [q for q in range(3) if q not in pair][0], M=Mv)
            s, E = sysm.to_reg(r[sub], v[sub])
            rho0 = tb_lc.rho_of_u(s[:, 0:2])
            r0n = np.sqrt(np.einsum('sk,sk->s', rho0, rho0))
            dtau = float(eta * np.median(np.sqrt(np.maximum(r0n, 1e-30) / sysm.mij)))

            for _ in range(max_steps):
                done = s[:, 8] >= dt_left
                if done.all():
                    break
                h = (dtau * (~done)).astype(float)[:, None]
                k1 = sysm.deriv(s, E)
                k2 = sysm.deriv(s + 0.5 * h * k1, E)
                k3 = sysm.deriv(s + 0.5 * h * k2, E)
                k4 = sysm.deriv(s + h * k3, E)
                s = s + (h / 6.0) * (k1 + 2 * k2 + 2 * k3 + k4)
                rr = tb_lc.rho_of_u(s[:, 0:2])
                dmin[sub] = np.minimum(dmin[sub],
                                       np.sqrt(np.einsum('sk,sk->s', rr, rr)))

            rc, vc = sysm.to_cartesian(s)
            # rebuild in the global COM frame (translation-invariant, so only offsets matter)
            com = (Mv[:, None] * rc).sum(axis=1) / Mv.sum()
            vcom = (Mv[:, None] * vc).sum(axis=1) / Mv.sum()
            r[sub] = rc - com[:, None, :]
            v[sub] = vc - vcom[:, None, :]
            t[sub] = t[sub] + np.minimum(s[:, 8], dt_left)

    drift = np.abs((tb.energy(r, v, 0.0) - E0) / np.maximum(np.abs(E0), 1e-30))
    return dict(r=r, v=v, drift=drift, dmin=dmin, t=t, switches=switches)
