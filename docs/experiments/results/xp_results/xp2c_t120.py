"""
EXPERIMENT 2 -- the t=120 horizon.

Ordered by information value, not by the region list, because this horizon is
expensive enough that partial completion is the likely outcome:

  near-field   escape went 0.00 -> 0.01 -> 0.56 across t=13/30/60. This is the
               ONLY region whose escape fraction crosses the 0.3 threshold, and
               its t=60 child timed out, so no exponent exists for it yet.
               Highest value by a wide margin.
  mid-field    already 1.00 everywhere and CHEAP (15-19 s at t=60) because early
               ionisation leaves a binary plus a receding escaper. Near-free.
  body2 mid    1.00 at t=30; parent timed out at t=60.
  far r~10 }   timed out at BOTH t=30 and t=60. Expected to time out again;
  body1 slice} run last so they cannot starve the informative regions.

Timeout raised to 1800 s: at t=60 the one long success took 688 s, so a 1200 s
cap at double the horizon would fail almost everything and tell us nothing.
n_sync=300 holds the sync interval at 0.4, matching the other horizons.
"""
import xp_driver

ORDER = [
    ('near-field',   1.0,  3.0, 0),
    ('mid-field',    1.0,  6.0, 0),
    ('body2 mid',    1.0, -5.0, 2),
    ('far r~10',     1.0, 13.0, 0),
    ('body1 slice', -2.0, -1.0, 1),
]
T, N_SYNC, HALVES, TIMEOUT = 120.0, 300, [0.05, 0.025], 1800

if __name__ == '__main__':
    store = xp_driver.Store('exp2_raw.json')     # same store; skips what exists
    i, total = 0, len(ORDER) * len(HALVES)
    for name, cx, cy, bd in ORDER:
        for half in HALVES:
            i += 1
            r = xp_driver.probe(store, timeout=TIMEOUT, config='burrau', cx=cx,
                                cy=cy, body=bd, half=half, jf=0.5, t=T,
                                eta=0.02, n_sync=N_SYNC)
            print(f"[{i}/{total}] t=120 {name} half={half} "
                  f"{r.get('_status')} {r.get('_seconds')}s", flush=True)
        print(f"--- {name} complete, flushed", flush=True)
    print("EXP2C t=120 DONE", flush=True)
