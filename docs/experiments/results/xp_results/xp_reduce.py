"""
Gating and reduction. Kept separate from the probe so that one expensive
integration can be re-reduced at many gate thresholds without re-running.

THE DRIFT GATE (brief section 3.1) is applied here and nowhere else:
    good = drift < thr,  and each footprint needs >= MIN_COPIES survivors.
NaN drift fails `drift < thr` automatically, so blown-up trajectories are
excluded rather than propagating.
"""
import numpy as np
import estimators

MIN_COPIES = 3


def gate(rec, thr=1e-3):
    d = np.asarray(rec['drift'], float)
    return d < thr                      # NaN -> False, which is what we want


def footprint_stats(rec, thr=1e-3, min_copies=MIN_COPIES):
    """Per-footprint contributors, normalised to [0,1]. Returns a list of dicts
    (one per SURVIVING footprint) plus retention bookkeeping."""
    gid = np.asarray(rec['gid'])
    good = gate(rec, thr)
    n_hat = np.asarray(rec['n_hat'], float)
    joint = np.asarray(rec['joint'])
    KE = np.asarray(rec['KE'], float)
    E = np.asarray(rec['E_fin'], float)
    nfp = int(gid.max()) + 1

    out = []
    for k in range(nfp):
        s = np.nonzero((gid == k) & good)[0]
        if len(s) < min_copies:
            continue
        nh = n_hat[s]
        shape_raw = float(np.mean(np.linalg.norm(nh - nh.mean(axis=0), axis=1)))
        # event: fraction not in the modal class, over SURVIVING copies. The
        # attainable maximum for n survivors is 1 - 1/n, so normalise by that.
        _, cnt = np.unique(joint[s], return_counts=True)
        disagree = 1.0 - cnt.max() / len(s)
        denom = 1.0 - 1.0 / len(s)
        out.append(dict(
            k=k, n_surv=int(len(s)),
            shape_raw=shape_raw,      shape_norm=shape_raw / 2.0,
            event_raw=float(disagree), event_norm=float(disagree / denom) if denom > 0 else 0.0,
            KE_raw=float(np.std(KE[s])), KE_norm=float(np.std(KE[s]) / 2.0),
            E_raw=float(np.std(E[s])),           # control, not a contributor
        ))
    return dict(
        footprints=out,
        n_footprints_total=nfp,
        n_footprints_surviving=len(out),
        retained_frac=float(good.mean()),
        drift_med=float(np.nanmedian(np.asarray(rec['drift'], float))),
        drift_max=float(np.nanmax(np.asarray(rec['drift'], float))),
        n_nan_drift=int(np.sum(~np.isfinite(np.asarray(rec['drift'], float)))),
    )


CONTRIBUTORS = ['shape', 'event', 'KE']


def quad_summary(rec, thr=1e-3):
    """Quad-level means and winner shares over surviving footprints."""
    st = footprint_stats(rec, thr)
    fps = st['footprints']
    if not fps:
        return dict(st, mean={}, wins={}, n_scored=0)
    mean = {c: float(np.mean([f[c + '_norm'] for f in fps])) for c in CONTRIBUTORS}
    mean_raw = {c: float(np.mean([f[c + '_raw'] for f in fps])) for c in CONTRIBUTORS}
    mean['E_control_spread'] = float(np.mean([f['E_raw'] for f in fps]))
    wins = {c: 0 for c in CONTRIBUTORS}
    ties = 0
    for f in fps:
        vals = {c: f[c + '_norm'] for c in CONTRIBUTORS}
        m = max(vals.values())
        winners = [c for c, v in vals.items() if v >= m - 1e-15]
        if len(winners) > 1:
            ties += 1
        wins[winners[0]] += 1                       # deterministic tie-break
    n = len(fps)
    return dict(st, mean=mean, mean_raw=mean_raw,
                wins={c: wins[c] / n for c in CONTRIBUTORS},
                wins_count=wins, n_ties=ties, n_scored=n)


def spread_vs_delta(recs, field, thr=1e-3):
    """(deltas, spreads) for one quad across jitter scales. `field` is one of
    shape/event/KE/E. Spread is the mean over surviving footprints."""
    ds, ss = [], []
    for r in sorted(recs, key=lambda r: r['delta']):
        st = footprint_stats(r, thr)
        if not st['footprints']:
            continue
        key = 'E_raw' if field == 'E' else field + '_raw'
        ds.append(r['delta'])
        ss.append(float(np.mean([f[key] for f in st['footprints']])))
    return np.array(ds), np.array(ss)


def alphas(recs, field, thr=1e-3):
    """Exponent by several estimators. ols2 on the two scales bracketing the
    reference jf=0.5 is what production can actually do; theil over all scales
    is the accuracy check."""
    d, s = spread_vs_delta(recs, field, thr)
    if len(d) < 2:
        return dict(ols2=np.nan, theil=np.nan, ols_all=np.nan, n_scales=len(d))
    # bracketing pair: the two scales closest to the reference
    return dict(
        ols2=estimators.est_ols(d[:2], s[:2]) if len(d) == 2 else estimators.est_ols(d[1:3], s[1:3]),
        theil=estimators.est_theil(d, s),
        ols_all=estimators.est_ols(d, s),
        n_scales=int(len(d)),
        deltas=d.tolist(), spreads=s.tolist(),
    )


def trustworthy(alpha_E, tol=0.05):
    return bool(np.isfinite(alpha_E) and abs(alpha_E - 1.0) <= tol)
