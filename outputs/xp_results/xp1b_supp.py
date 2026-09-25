"""
EXPERIMENT 1 supplement -- more eq_rot quads.

The L!=0,E<0 arm landed at exactly 0.50 KE win-share on only 3 trustworthy
quads, which is the falsification boundary; and the drift gate removed all three
'core' quads, so the surviving sample was biased toward large radius. These fill
the radial gap between core and outer.
"""
import xp_driver

# Radially outward from the COM (origin) along each body's own direction.
REGIONS = [
    ('b0 r1.6',  0.00,  1.60, 0), ('b0 r2.2',  0.00,  2.20, 0),
    ('b1 r1.6', -1.43, -0.72, 1), ('b1 r2.2', -1.97, -0.98, 1),
    ('b2 r1.6',  1.43, -0.72, 2), ('b2 r2.2',  1.97, -0.98, 2),
]
JFS = [0.125, 0.25, 0.5, 1.0]

if __name__ == '__main__':
    store = xp_driver.Store('exp1_raw.json')     # same store; report picks it up
    for name, cx, cy, bd in REGIONS:
        for jf in JFS:
            r = xp_driver.probe(store, timeout=300, config='eq_rot', cx=cx, cy=cy,
                                body=bd, jf=jf, t=13.0, eta=0.01, n_sync=32)
            print(f"eq_rot/{name} jf={jf} {r.get('_status')} {r.get('_seconds')}s", flush=True)
        print(f"--- {name} complete, flushed", flush=True)
    print("EXP1B PROBES DONE", flush=True)
