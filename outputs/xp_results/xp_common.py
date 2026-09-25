"""
Shared machinery for the three refinement experiments.

NEW FILE -- the harness (tb, tb_az, tb_all_az, sweep_az, estimators, est_run,
refine_test) is NOT modified. Everything here either calls it or patches its
module globals, as the brief directs.

Conventions enforced here (brief section 3):
  1. drift gate applied before ANY spread; >=3 surviving copies per footprint
  2. total-energy control carried everywhere (alpha_E, true value exactly 1.0)
  3. jitter scales with the cell (burrau_grid does this; we only vary jitter_frac)
  4. AZ integrator throughout
  5. n reported (regions / samples / copies)
"""
import numpy as np

# ---------------------------------------------------------------- configs ----
# Each config patches tb.M / tb.R0 (module globals) and supplies initial
# velocities. burrau_grid always starts from rest, so v0 is overwritten after.

CONFIGS = {
    'burrau': dict(
        label='Burrau m=(3,4,5) at rest',
        M=[3.0, 4.0, 5.0],
        R0=[[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
        omega=0.0,
        regime='L=0, E<0',
    ),
    'eq_rot': dict(
        label='equal mass, rigid rotation omega=0.3',
        M=[1.0, 1.0, 1.0],
        R0=[[0.0, 1.0], [-1.0, -0.5], [1.0, -0.5]],
        omega=0.3,
        regime='L!=0, E<0',
    ),
    'eq_fast': dict(
        label='equal mass, rigid rotation omega=1.2',
        M=[1.0, 1.0, 1.0],
        R0=[[0.0, 1.0], [-1.0, -0.5], [1.0, -0.5]],
        omega=1.2,
        regime='L!=0, E>0',
    ),
}

# Region centres. Burrau: the canonical list, minus 'deep interior'
# (centre (0,0) body 0) which is a near-triple encounter -- skipped per brief.
REGIONS_BURRAU = [
    ('near-field',   1.0,   3.0,  0),
    ('mid-field',    1.0,   6.0,  0),
    ('far r~10',     1.0,  13.0,  0),
    ('body2 core',   1.0,  -1.0,  2),
    ('body2 mid',    1.0,  -5.0,  2),
    ('body1 slice', -2.0,  -1.0,  1),
    ('body1 far',   -2.0,  -7.0,  1),
]

# Equal-mass geometry: COM is at the origin, so the analogue of the Burrau
# list is each body's own position ('core') plus a radially outward offset.
REGIONS_EQUAL = [
    ('b0 core',   0.0,   1.0,  0),
    ('b0 outer',  0.0,   3.0,  0),
    ('b1 core',  -1.0,  -0.5,  1),
    ('b1 outer', -2.5,  -1.25, 1),
    ('b2 core',   1.0,  -0.5,  2),
    ('b2 outer',  2.5,  -1.25, 2),
]

# Experiment 1 supplement: intermediate radii, added because the L!=0,E<0 arm
# was borderline on only 3 trustworthy quads (see xp1b_supp.py).
REGIONS_EQUAL_SUPP = [
    ('b0 r1.6',  0.00,  1.60, 0), ('b0 r2.2',  0.00,  2.20, 0),
    ('b1 r1.6', -1.43, -0.72, 1), ('b1 r2.2', -1.97, -0.98, 1),
    ('b2 r1.6',  1.43, -0.72, 2), ('b2 r2.2',  1.97, -0.98, 2),
]


def regions_for(config):
    return REGIONS_BURRAU if config == 'burrau' else (REGIONS_EQUAL + REGIONS_EQUAL_SUPP)


def apply_config(name):
    """Patch the harness module globals in place. Must be called before any
    other harness import that caches them (refine_test caches MT/MTOT)."""
    import tb
    cfg = CONFIGS[name]
    tb.M = np.array(cfg['M'], float)
    tb.R0 = np.array(cfg['R0'], float)
    tb.V0 = np.zeros((3, 2))
    import refine_test as R
    R.MT, R.MTOT = tb.M, tb.M.sum()          # refine_test caches these
    return cfg


def rigid_rotation(r0, omega):
    """v = omega x (r - R_com), per-sample (each sample has its own COM because
    the slice moves one body's position)."""
    import tb
    M = tb.M
    com = (M[None, :, None] * r0).sum(axis=1, keepdims=True) / M.sum()
    d = r0 - com
    v = np.zeros_like(r0)
    v[..., 0] = -omega * d[..., 1]
    v[..., 1] = omega * d[..., 0]
    return v


def angular_momentum(r, v):
    import tb
    return (tb.M[None, :] * (r[..., 0] * v[..., 1] - r[..., 1] * v[..., 0])).sum(axis=1)


# ------------------------------------------------------------------ probe ----
def run_probe(config, cx, cy, body, half=0.05, jf=0.5, t=13.0, N=4, ens=7,
              eta=0.01, n_sync=32, seed=0, max_steps=30000):
    """One quad. Returns per-TRAJECTORY raw arrays -- no gating, no reduction.
    Gating and reduction happen downstream so a single expensive integration can
    be re-analysed at many gate thresholds (Experiment 3) without re-running."""
    import tb, tb_all_az as AA, refine_test as R

    cfg = apply_config(config)
    r0, v0, gid, _, hx = tb.burrau_grid(N, N, cx, cy, half, body=body, ens=ens,
                                        jitter_frac=jf, seed=seed)
    if cfg['omega'] != 0.0:
        v0 = rigid_rotation(r0, cfg['omega'])

    E_init = tb.energy(r0, v0, 0.0)
    L_init = angular_momentum(r0, v0)

    res = AA.integrate_all_az(r0, v0, t_max=t, n_sync=n_sync, eta=eta,
                              max_steps=max_steps)

    n_hat = R.shape_vec(res['r'])
    cls = tb.classify(dict(r=res['r'], v=res['v']))
    joint = (cls < 3).astype(int) * 3 + res['binary_id']
    KE = 0.5 * np.einsum('k,nki->n', tb.M, res['v'] * res['v'])
    E_fin = tb.energy(res['r'], res['v'], 0.0)

    return dict(
        config=config, cx=cx, cy=cy, body=body, half=half, jf=jf, t=t,
        N=N, ens=ens, eta=eta, n_sync=n_sync, seed=seed, hx=float(hx),
        # delta: the physical perturbation scale. jitter is U(-jf*hx, +jf*hx),
        # so delta ∝ jf*hx. hx is fixed for fixed (half, N).
        delta=float(jf * hx),
        E_init=E_init.tolist(), L_init=L_init.tolist(),
        gid=gid.tolist(),
        drift=res['drift'].tolist(),
        n_hat=n_hat.tolist(),
        joint=joint.tolist(),
        KE=KE.tolist(),
        E_fin=E_fin.tolist(),
        t_end=res['t_end'].tolist(),
        censored=res['censored'].tolist(),
        ftle=np.asarray(res['ftle'], float).tolist(),
        dmin=res['dmin'].tolist(),
    )
