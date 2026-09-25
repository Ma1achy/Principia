"""
EXPERIMENT 3 -- calibrate the drift gate.

Claim under test: 1e-3 is a reasonable gate threshold.

Method: for each region x jitter scale x integrator tolerance, integrate ONCE
and store per-trajectory drift. Then apply every candidate threshold offline and
score it on
    |alpha_E - 1|   the control error (lower better; alpha_E is known == 1.0)
    retained        fraction of trajectories surviving (higher better)
"""
import json, numpy as np
import xp_driver, xp_reduce

# Regions chosen to span the observed drift range: near-field is benign
# (max drift 8e-05), body2 core is the stress case (max drift 1.4e+02).
REGIONS = [
    ('burrau', 'near-field',   1.0,  3.0, 0),
    ('burrau', 'mid-field',    1.0,  6.0, 0),
    ('burrau', 'body2 core',   1.0, -1.0, 2),
    ('burrau', 'body2 mid',    1.0, -5.0, 2),
    ('burrau', 'body1 far',   -2.0, -7.0, 1),
    ('eq_rot', 'b0 core',      0.0,  1.0, 0),   # L!=0 stress case, retention 0.56
]
JFS = [0.125, 0.25, 0.5, 1.0, 2.0]
ETAS = [0.02, 0.01, 0.005]
THRESHOLDS = [1e-2, 1e-3, 1e-4, 1e-5, 1e-6]

if __name__ == '__main__':
    store = xp_driver.Store('exp3_raw.json')
    total = len(REGIONS) * len(JFS) * len(ETAS)
    i = 0
    for cfg, name, cx, cy, bd in REGIONS:
        for eta in ETAS:
            for jf in JFS:
                i += 1
                r = xp_driver.probe(store, timeout=240, config=cfg, cx=cx, cy=cy,
                                    body=bd, jf=jf, t=13.0, eta=eta, n_sync=32)
                print(f"[{i}/{total}] {cfg}/{name} eta={eta} jf={jf} "
                      f"{r.get('_status')} {r.get('_seconds')}s", flush=True)
        print(f"--- region {name} complete, flushed to {store.path}", flush=True)
    print("EXP3 PROBES DONE", flush=True)
