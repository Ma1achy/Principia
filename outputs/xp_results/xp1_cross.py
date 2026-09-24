"""
EXPERIMENT 1 -- cross-system: does the conclusion survive L != 0 and E > 0?

Claim under test: kinetic energy is the dominant contributor to ensemble_spread,
winning the max in 69-100% of footprints.

Three configurations:
  burrau   m=(3,4,5) at rest          L=0,    E<0   (the published baseline)
  eq_rot   equal mass, omega=0.3      L!=0,   E<0
  eq_fast  equal mass, omega=1.2      L!=0,   E>0

NOTE: equal-mass AT REST was dropped as the L=0 arm. R0=[[0,1],[-1,-.5],[1,-.5]]
has sides 1.803/1.803/2.000 -- near-equilateral -- so equal masses released from
rest collapse homothetically toward a TRIPLE collision, which AZ provably cannot
regularise (brief 2.2). Measured: drift NaN, retention 0.17-0.26, 235-354 s per
probe. Burrau is the correct L=0 baseline anyway: it is the configuration the
claim was actually measured on.

Contributors are read at jf=0.5; the extra jitter scales exist to fit alpha_E.
"""
import xp_common, xp_driver

JFS = [0.125, 0.25, 0.5, 1.0]
REF_JF = 0.5

if __name__ == '__main__':
    store = xp_driver.Store('exp1_raw.json')
    plan = [(c, n, cx, cy, bd)
            for c in ['burrau', 'eq_rot', 'eq_fast']
            for (n, cx, cy, bd) in xp_common.regions_for(c)]
    total = len(plan) * len(JFS)
    i = 0
    for cfg, name, cx, cy, bd in plan:
        for jf in JFS:
            i += 1
            r = xp_driver.probe(store, timeout=240, config=cfg, cx=cx, cy=cy,
                                body=bd, jf=jf, t=13.0, eta=0.01, n_sync=32)
            print(f"[{i}/{total}] {cfg}/{name} jf={jf} {r.get('_status')} "
                  f"{r.get('_seconds')}s", flush=True)
        print(f"--- {cfg}/{name} complete, flushed", flush=True)
    print("EXP1 PROBES DONE", flush=True)
