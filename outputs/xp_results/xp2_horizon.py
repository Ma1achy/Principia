"""
EXPERIMENT 2 -- long horizon: is t_end permanently excluded, or just mis-scoped?

Claim under test: t_end carries no usable signal -- 100% censored in three of
four regions at t=13.

Escape fraction vs horizon, t in {13,30,60,120}. eta=0.02 per the brief, and
n_sync scaled with t to hold the sync interval near 0.4 time units.

Where escape fraction > 0.3 we also take the t_end spread over ESCAPED copies
only, parent (half=0.05) vs child (half=0.025), for its exponent.
"""
import xp_driver

REGIONS = [
    ('burrau', 'near-field',   1.0,  3.0, 0),   # 100% censored at t=13
    ('burrau', 'mid-field',    1.0,  6.0, 0),   # 0% censored at t=13
    ('burrau', 'far r~10',     1.0, 13.0, 0),   # 100% censored at t=13
    ('burrau', 'body2 mid',    1.0, -5.0, 2),   # 1% censored at t=13
    ('burrau', 'body1 slice', -2.0, -1.0, 1),   # 100% censored at t=13
]
HORIZONS = [13.0, 30.0, 60.0, 120.0]
SYNC_INTERVAL = 0.4
HALVES = [0.05, 0.025]      # parent, child -- for the t_end exponent


def n_sync_for(t):
    return max(8, int(round(t / SYNC_INTERVAL)))


if __name__ == '__main__':
    store = xp_driver.Store('exp2_raw.json')
    # HORIZON-MAJOR ordering. At t=60 a few trajectories go non-finite and burn
    # the integrator step budget (measured: 46 s vs ~10 s linear, 3/128 never
    # reaching t_max -- they carry NaN drift and are gated out, so the numbers
    # stay valid but the cost does not). t=120 is therefore run LAST, so that
    # losing it still leaves every horizon the falsification criterion needs.
    total = len(HORIZONS) * len(REGIONS) * len(HALVES)
    i = 0
    for t in HORIZONS:
        for cfg, name, cx, cy, bd in REGIONS:
            for half in HALVES:
                i += 1
                to = 300 if t <= 30 else (700 if t <= 60 else 1200)
                r = xp_driver.probe(store, timeout=to, config=cfg, cx=cx, cy=cy,
                                    body=bd, half=half, jf=0.5, t=t, eta=0.02,
                                    n_sync=n_sync_for(t))
                print(f"[{i}/{total}] t={t} {name} half={half} "
                      f"{r.get('_status')} {r.get('_seconds')}s", flush=True)
        print(f"=== horizon t={t} complete, flushed ===", flush=True)
    print("EXP2 PROBES DONE", flush=True)
