"""Subprocess entry point: run ONE probe, write JSON, exit.

Isolation matters: when the AZ state overflows to NaN, `done = s[:,8] >= dt_left`
is never satisfied, so integrate_az burns its whole step budget (observed: 354 s
against ~3 s nominal). Running each probe in its own process lets the driver
impose a hard wall-clock timeout without touching the harness.
"""
import json, sys
import xp_common

if __name__ == '__main__':
    args = json.loads(sys.argv[1])
    outpath = args.pop('_out')
    rec = xp_common.run_probe(**args)
    with open(outpath, 'w') as f:
        json.dump(rec, f)
