"""Experiment 1 analysis: contributor dominance across L and E regimes."""
import json, numpy as np, xp_common, xp_reduce

recs = json.load(open('xp_results/exp1_raw.json'))
ok = [r for r in recs if not r.get('_failed')]
THR, REF_JF = 1e-3, 0.5

NAME = {}
for c in xp_common.CONFIGS:
    for n, cx, cy, bd in xp_common.regions_for(c):
        NAME[(c, cx, cy, bd)] = n


def qkey(r):
    return (r['config'], r['cx'], r['cy'], r['body'])


rows = []
for k in sorted({qkey(r) for r in ok}):
    grp = [r for r in ok if qkey(r) == k]
    ref = next((r for r in grp if r['jf'] == REF_JF), None)
    if ref is None:
        continue
    s = xp_reduce.quad_summary(ref, THR)
    aE = xp_reduce.alphas(grp, 'E', THR)
    Ei = np.asarray(ref['E_init'], float)
    Li = np.asarray(ref['L_init'], float)
    rows.append(dict(
        config=k[0], region=NAME[k], n_scored=s['n_scored'],
        retained=s['retained_frac'], drift_med=s['drift_med'], drift_max=s['drift_max'],
        mean=s['mean'], mean_raw=s['mean_raw'], wins=s['wins'], wins_count=s['wins_count'],
        n_ties=s['n_ties'],
        alpha_E=aE['ols2'], alpha_E_theil=aE['theil'],
        trust=xp_reduce.trustworthy(aE['ols2']),
        E0=[float(Ei.min()), float(Ei.max())], L0=[float(Li.min()), float(Li.max())],
    ))
json.dump(rows, open('xp_results/exp1_table.json', 'w'), indent=1)

print("## Per-quad contributor dominance (jf=0.5, t=13, gate 1e-3, 16 samples x 8 copies)\n")
print("| config | region | E0 range | L0 | fp scored | mean shape | mean event | mean KE | "
      "win shape | win event | **win KE** | alpha_E (ols2) | trust |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    t = 'ok' if r['trust'] else '**UNTRUSTWORTHY**'
    print(f"| {r['config']} | {r['region']} | [{r['E0'][0]:+.2f},{r['E0'][1]:+.2f}] | "
          f"{r['L0'][0]:+.2f} | {r['n_scored']}/16 | "
          f"{r['mean']['shape']:.4f} | {r['mean']['event']:.4f} | {r['mean']['KE']:.4f} | "
          f"{r['wins']['shape']:.2f} | {r['wins']['event']:.2f} | **{r['wins']['KE']:.2f}** | "
          f"{r['alpha_E']:.4f} | {t} |")

print("\n## Pooled by configuration (footprint-weighted, TRUSTWORTHY quads only)\n")
print("| config | regime | quads used / total | footprints | mean shape | mean event | mean KE | "
      "win shape | win event | **win KE** |")
print("|---|---|---|---|---|---|---|---|---|---|")
for c in ['burrau', 'eq_rot', 'eq_fast']:
    sub = [r for r in rows if r['config'] == c]
    good = [r for r in sub if r['trust']]
    if not good:
        print(f"| {c} | {xp_common.CONFIGS[c]['regime']} | 0/{len(sub)} | - | - | - | - | - | - | - |")
        continue
    n = sum(r['n_scored'] for r in good)
    w = {k: sum(r['wins_count'][k] for r in good) / n for k in xp_reduce.CONTRIBUTORS}
    m = {k: sum(r['mean'][k] * r['n_scored'] for r in good) / n for k in xp_reduce.CONTRIBUTORS}
    print(f"| {c} | {xp_common.CONFIGS[c]['regime']} | {len(good)}/{len(sub)} | {n} | "
          f"{m['shape']:.4f} | {m['event']:.4f} | {m['KE']:.4f} | "
          f"{w['shape']:.2f} | {w['event']:.2f} | **{w['KE']:.2f}** |")

print("\n## Same, ALL quads including untrustworthy (sensitivity check)\n")
print("| config | quads | footprints | win shape | win event | **win KE** |")
print("|---|---|---|---|---|---|")
for c in ['burrau', 'eq_rot', 'eq_fast']:
    sub = [r for r in rows if r['config'] == c]
    n = sum(r['n_scored'] for r in sub)
    if not n:
        continue
    w = {k: sum(r['wins_count'][k] for r in sub) / n for k in xp_reduce.CONTRIBUTORS}
    print(f"| {c} | {len(sub)} | {n} | {w['shape']:.2f} | {w['event']:.2f} | **{w['KE']:.2f}** |")

print("\n## Raw (un-normalised) contributor magnitudes\n")
print("| config | region | shape raw | event raw | KE raw | E control spread |")
print("|---|---|---|---|---|---|")
for r in rows:
    print(f"| {r['config']} | {r['region']} | {r['mean_raw']['shape']:.4e} | "
          f"{r['mean_raw']['event']:.4e} | {r['mean_raw']['KE']:.4e} | "
          f"{r['mean']['E_control_spread']:.4e} |")
