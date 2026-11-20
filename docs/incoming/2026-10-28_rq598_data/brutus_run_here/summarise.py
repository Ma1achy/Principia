#!/usr/bin/env python3
"""Summarise the pilot: cost per point (median, spread), scaling fits, and the projections, all from the raw CSVs.
Every number printed is either MEASURED (taken from a CSV) or ESTIMATED (a model applied to measured numbers); the
label is in the line. "Brutus, run here": ours, not the papers'.

usage: summarise.py <gg1 dir> <br1 dir> <Fig. 1 left-panel CSV (used only for the lifetime distribution weights)>
"""
import csv
import math
import statistics as st
import sys

import numpy as np

gg, br, fig = sys.argv[1:4]


def rd(p):
    return list(csv.DictReader(open(p)))


def q(v, p):
    return float(np.percentile(v, p))


# ---------------------------------------------------------------- GG-1 sample, eps = 1e-6, L_w = 1024
rows = [r for r in rd(f"{gg}/pilot_eps1e-6_Lw1024.csv")]
print(f"== GG-1, eps = 1e-6, L_w = 1024 bits, T capped at 1000: {len(rows)} points (MEASURED)")
esc = [r for r in rows if r["status"] == "escaped"]
oth = [r for r in rows if r["status"] != "escaped"]
print(f"escaped: {len(esc)}   other: {[(r['i'], r['j'], r['status'], r['t_reached'], r['wall_fwd_s']) for r in oth]}")
wf = [float(r["wall_fwd_s"]) for r in rows]
wr = [float(r["wall_rev_s"]) for r in esc]
print(f"forward wall s  : median {st.median(wf):.1f}  mean {st.mean(wf):.1f}  min {min(wf):.1f}  max {max(wf):.1f}  "
      f"p10 {q(wf, 10):.1f}  p90 {q(wf, 90):.1f}")
print(f"reverse wall s  : median {st.median(wr):.1f}  mean {st.mean(wr):.1f}  min {min(wr):.1f}  max {max(wr):.1f}")
tot = [float(r["wall_fwd_s"]) + float(r["wall_rev_s"]) for r in esc]
print(f"fwd+rev wall s  : median {st.median(tot):.1f}  mean {st.mean(tot):.1f}  (escaped points)")
steps = [int(r["steps_fwd"]) for r in rows]
print(f"forward steps   : median {st.median(steps)}  min {min(steps)}  max {max(steps)}")
mps = [1e3 * float(r["wall_fwd_s"]) / int(r["steps_fwd"]) for r in rows]
print(f"ms per step     : median {st.median(mps):.2f}  min {min(mps):.2f}  max {max(mps):.2f}")
rate = [float(r["wall_fwd_s"]) / float(r["t_reached"]) for r in rows]
print(f"wall s per unit of simulated time (forward): median {st.median(rate):.2f}  mean {st.mean(rate):.2f}  "
      f"p10 {q(rate, 10):.2f}  p90 {q(rate, 90):.2f}  max {max(rate):.2f}")
rev = {}
for r in esc:
    rev[r["reversal"]] = rev.get(r["reversal"], 0) + 1
print("reversibility test (log10 delta < -3):", rev, " (MEASURED; at eps = 1e-6 the paper's Fig. 2 puts f_irr near 1)")
print(f"energy relative error at escape: median {st.median(abs(float(r['energy_relerr'])) for r in esc):.2e}")

# ---------------------------------------------------------------- GG-1 projection of the full panel
fl = rd(fig)
Ts = [1000.0 if r["log10_T"].startswith("censored") else 10 ** float(r["log10_T"]) for r in fl]
nblack = sum(1 for r in fl if r["log10_T"].startswith("censored"))
tsum = sum(Ts)
print(f"\n== GG-1 full eps = 1e-6 panel, projected (ESTIMATE)")
print(f"lifetime weights: the paper's own Fig. 1 left panel, its values read as OUR time units (not converted; "
      f"hm_units.py gives the Heggie-Mathieu conversion): "
      f"sum of T over 1212 points = {tsum:.0f} (the {nblack} black points counted at 1000, {1000 * nblack} of it)")
ratio = [float(r["t_reached"]) / float(r["T"]) for r in esc if float(r["T"]) > 0]
print(f"integrated time / lifetime T at escape in our sample (MEASURED): median {st.median(ratio):.2f}  "
      f"range {min(ratio):.2f}-{max(ratio):.2f}")
revfac = st.median(float(r["wall_rev_s"]) / float(r["wall_fwd_s"]) for r in esc)
print(f"reverse leg / forward leg wall (MEASURED, median): {revfac:.2f}")
for lab, rt in (("low (median rate)", st.median(rate)), ("mid (mean rate)", st.mean(rate)), ("high (p90 rate)", q(rate, 90))):
    for tlab, tf in (("integrated time = 1.0 x T", 1.0), ("integrated time = median ratio x T", st.median(ratio))):
        fwd = rt * tf * tsum
        print(f"  {lab:18s} {tlab:36s}: forward only {fwd / 3600:7.1f} core-hours; with reversal {fwd * (1 + revfac) / 3600:7.1f} core-hours")

# ---------------------------------------------------------------- GG-1 scaling probes
try:
    sc = rd(f"{gg}/scaling_gg1.csv")
except FileNotFoundError:
    sc = []
if sc:
    print("\n== GG-1 scaling probes (MEASURED; eps >= 1e-6 only)")
    for r in sc:
        print(f"eps={r['eps']:>5s} Lw={r['Lw']:>4s} ({r['i']},{r['j']}) {r['status']:9s} t_reached {float(r['t_reached']):8.2f} steps {r['steps_fwd']:>6s} "
              f"wall {float(r['wall_fwd_s']):7.1f}s  ms/step {1e3 * float(r['wall_fwd_s']) / max(1, int(r['steps_fwd'])):6.2f}  s per unit t {float(r['wall_fwd_s']) / max(1e-9, float(r['t_reached'])):6.2f}")

# ---------------------------------------------------------------- BR-1
b = rd(f"{br}/results_br1.csv")
print("\n== BR-1 (MEASURED): wall s per run to t = 10 (the 9 full-length runs), by (eps, L_w)")
cfg = {}
for r in b:
    if not r["stratum"].startswith("full"):
        continue
    cfg.setdefault((float(r["eps"]), int(r["Lw"])), []).append(r)
for k in sorted(cfg, reverse=True):
    v = cfg[k]
    ok = [r for r in v if r["status"] == "ok"]
    w = [float(r["wall_s"]) for r in ok]
    s = [int(r["steps"]) for r in ok]
    print(f"eps={k[0]:g} Lw={k[1]:3d}: {len(ok)}/{len(v)} ok  wall median {st.median(w):.2f} min {min(w):.2f} max {max(w):.2f}  "
          f"steps median {st.median(s)}  ms/step median {st.median(1e3 * a / c for a, c in zip(w, s)):.3f}")
