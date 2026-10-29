#!/usr/bin/env python3
"""Convert the pilot's GG-1 numbers to Heggie-Mathieu (HM) N-body units, and redo the comparison with Fig. 1 and the
cost projections in them (review finding F1, F2, F5, F6, F8 on PR #334). "Brutus, run here": ours, not the papers'.

Units. The pilot runs in m = 1 per body, G = 1, r12 = 1 (bodies 1 and 2 at (-1/2, 0), (1/2, 0); body 3 at (i/64, j/64);
all at rest). Boekholt, Portegies Zwart & Valtonen 2020, Appendix A, give times and distances in "time units (Heggie &
Mathieu 1986)": G = 1, total mass M = 1, total energy E = -1/4. With G = 1 in both, a mass scale mu = 3 (our masses per
HM mass) and a length scale l (our lengths per HM length) fix the time scale tau = sqrt(l^3 / mu). At rest
E_ours = -S with S = sum over pairs of 1/r_ij, and E scales as mu^2 / l, so -S = -(1/4) * 9 / l, l = 9 / (4 S).
Writing lam = 1/l = (4/9) S:
    T_HM = T_ours * sqrt(3 lam^3)        (the factor depends on the point)
    R_HM = R_ours * lam
The paper's Fig. 1 is read as HM units throughout; that it is in HM units at all is the paper's statement for its
Appendix A times, applied here to the figure (an assumption, stated as one wherever it is used).

Also writes <gg1 dir>/ours_vs_fig1.csv: the 16 sample points, ours in both unit systems, against route 2's CSVs
(`transcriptions/gg1_fig1/`, the one record of Fig. 1), whose values it copies verbatim.

usage: hm_units.py <gg1 dir> <route-2 left CSV> <route-2 right CSV>
"""
import csv
import math
import statistics as st
import sys

gg, left_p, right_p = sys.argv[1:4]


def rd(p):
    return list(csv.DictReader(open(p)))


def lam(i, j):
    x, y = i / 64, j / 64
    r12 = 1.0
    r13 = math.hypot(x + 0.5, y)
    r23 = math.hypot(x - 0.5, y)
    return (4 / 9) * (1 / r12 + 1 / r13 + 1 / r23)


def fac(i, j):
    return math.sqrt(3 * lam(i, j) ** 3)


def fig(p):
    out = {}
    for r in rd(p):
        v = r["log10_T"]
        out[(int(r["i"]), int(r["j"]))] = None if v.startswith("censored") else float(v)
    return out


L, R = fig(left_p), fig(right_p)
grid = sorted(L)
F = {k: fac(*k) for k in grid}
fs = sorted(F.values())
print("== The conversion factor T_HM / T_ours = sqrt(3 lam^3), lam = (4/9) sum 1/r_ij, over the 1212 grid points")
print(f"min {fs[0]:.3f}  median {st.median(fs):.3f}  max {fs[-1]:.3f}")
eq = math.sqrt(3 * (4 / 9 * 3) ** 3)
# equilateral, side 1, at rest: each body falls to the centre from rho0 = 1/sqrt(3) under k / rho^2, k = 1/sqrt(3);
# free-fall time (pi/2) sqrt(rho0^3 / (2 k)) = (pi/2) sqrt(1/6)
t_eq = (math.pi / 2) * math.sqrt(1 / 6)
print(f"check: equilateral triangle of side 1 at rest collapses at T = {t_eq:.4f} (ours) = {t_eq * eq:.4f} HM")

print("\n== The escape radius and the T cap, ours against the paper's (Appendix A: 10 distance units; runs to 10^4)")
ls = sorted(lam(*k) for k in grid)
print(f"our R_esc = 10 (our units) = {10 * ls[0]:.2f} to {10 * ls[-1]:.2f} HM units (median {10 * st.median(ls):.2f});"
      f" the paper's 10 HM units = {10 / ls[-1]:.2f} to {10 / ls[0]:.2f} of ours")
print(f"our T cap = 1000 (our units) = {1000 * fs[0]:.0f} to {1000 * fs[-1]:.0f} HM units; the paper's 10^4 HM ="
      f" {1e4 / fs[-1]:.0f} to {1e4 / fs[0]:.0f} of ours; its black class (T > 1000 HM) = T_ours > {1000 / fs[-1]:.1f}"
      f" to {1000 / fs[0]:.0f}")

print("\n== GG-1, eps = 1e-6, L_w = 1024, p_max = 4: ours converted to HM units against Fig. 1 (route 2)")
print("(i, j)   factor  T_ours    T_HM     log10 T_HM  Fig.1 left  Fig.1 right  ours_HM - left (dex)")
pil = {(int(r["i"]), int(r["j"])): r for r in rd(f"{gg}/pilot_eps1e-6_Lw1024.csv")}
for k in sorted(pil):
    t = float(pil[k]["T"])
    th = t * F[k]
    lg = math.log10(th)
    fl, fr = L[k], R[k]
    d = "censored" if fl is None else f"{lg - fl:+.2f}"
    print(f"{str(k):9s}{F[k]:6.2f}  {t:8.4f}  {th:8.3f}  {lg:9.3f}  {'>3' if fl is None else f'{fl:9.3f}':>10s}"
          f"  {'>3' if fr is None else f'{fr:9.3f}':>10s}  {d:>10s}")
lutL = {(int(r["i"]), int(r["j"])): r["lut_index"] for r in rd(left_p)}
lutR = {(int(r["i"]), int(r["j"])): r["lut_index"] for r in rd(right_p)}
raw_l = {(int(r["i"]), int(r["j"])): r["log10_T"] for r in rd(left_p)}
raw_r = {(int(r["i"]), int(r["j"])): r["log10_T"] for r in rd(right_p)}
with open(f"{gg}/ours_vs_fig1.csv", "w", newline="") as fo:
    wr = csv.writer(fo, lineterminator="\n")
    wr.writerow(["i", "j", "ours_T_eps1e-6_Lw1024_our_units", "ours_log10_T_our_units", "factor_T_HM_over_T_ours",
                 "ours_log10_T_HM", "route2_fig1_left_log10_T_eps1e-6", "route2_fig1_right_log10_T_eps1e-70",
                 "ours_HM_minus_route2_left_dex"])
    for k in sorted(pil):
        t = float(pil[k]["T"])
        lg = math.log10(t * F[k])
        wr.writerow([k[0], k[1], pil[k]["T"], f"{math.log10(t):.3f}", f"{F[k]:.4f}", f"{lg:.3f}", raw_l[k], raw_r[k],
                     "" if L[k] is None else f"{lg - L[k]:+.3f}"])
agree = sum(1 for k in grid if lutL[k] == lutR[k])
print(f"(points where the two panels share a LUT index, black included, over all 1212: {agree})")

print("\n== The three points where the panels agree in the 16-point sample, ours over eps = 1e-4 .. 1e-6 (L_w = 128), HM units")
rob = rd(f"{gg}/robustness_three_points_Lw128_norev.csv")
for k in ((1, 54), (14, 20), (3, 43)):
    ts = [float(r["T"]) * F[k] for r in rob if (int(r["i"]), int(r["j"])) == k]
    print(f"{k}: T_HM {min(ts):.4f}-{max(ts):.4f}, log10 {math.log10(min(ts)):.3f}; Fig. 1 (both panels) {L[k]:.3f};"
          f" difference {math.log10(min(ts)) - L[k]:+.2f} dex")

print("\n== (3, 43) and its neighbours in Fig. 1 (route 2), log10 T in HM units: left / right; and our factor there")
for k in ((3, 42), (3, 44), (2, 43), (4, 43), (2, 42), (2, 44), (4, 42), (4, 44), (3, 43)):
    f = lambda v: ">3" if v is None else f"{v:.3f}"
    print(f"{str(k):9s} {f(L[k]):>6s} / {f(R[k]):>6s}   factor {F[k]:.2f}")

print("\n== Weights for the projections: sum of Fig. 1 left lifetimes, in our time units (each T_HM / factor)")
w = {}
for cap_lab, cap in (("black at 1000 HM (#334's count)", 1000.0), ("black at 10^4 HM (the paper's run length)", 1e4)):
    raw = sum(cap if L[k] is None else 10 ** L[k] for k in grid)
    conv = sum((cap if L[k] is None else 10 ** L[k]) / F[k] for k in grid)
    w[cap] = conv
    print(f"{cap_lab:42s}: sum T read as our units {raw:9.0f};  converted from HM {conv:8.0f}  (ratio {raw / conv:.2f})")
raw1000 = sum(1000.0 if L[k] is None else 10 ** L[k] for k in grid)

pr = rd(f"{gg}/pilot_eps1e-6_Lw1024.csv")
esc = [r for r in pr if r["status"] == "escaped"]
rate = [float(r["wall_fwd_s"]) / float(r["t_reached"]) for r in pr]
ratio = st.median(float(r["t_reached"]) / float(r["T"]) for r in esc)
revfac = st.median(float(r["wall_rev_s"]) / float(r["wall_fwd_s"]) for r in esc)
print(f"\n== eps = 1e-6 panel, forward leg (ESTIMATE): rate median {st.median(rate):.2f} / mean {st.mean(rate):.2f} s per our"
      f" unit; integrated / T median {ratio:.2f}; reverse / forward {revfac:.2f}")
for lab, wt in (("as #334 (Fig. 1 read as our units, black 1000)", raw1000), ("HM-converted, black at 1000 HM", w[1000.0]),
                ("HM-converted, black at 10^4 HM", w[1e4])):
    lo = st.median(rate) * wt / 3600
    hi = st.mean(rate) * ratio * wt / 3600
    print(f"{lab:48s}: forward {lo:6.1f}-{hi:6.1f} core-hours; with reversal {lo * (1 + revfac):6.1f}-{hi * (1 + revfac):6.1f}")

print("\n== eps = 1e-70 (ESTIMATE): ms per step at p_max 8 and 16 from the same two measured points, point (1, 54)")
pm = rd(f"{gg}/below_1e6_pmax_probe.csv")
for p in ("8", "16"):
    a = {r["eps"]: r for r in pm if r["pmax"] == p}
    m8 = 1e3 * float(a["1e-8"]["wall_fwd_s"]) / int(a["1e-8"]["steps_fwd"])
    m10 = 1e3 * float(a["1e-10"]["wall_fwd_s"]) / int(a["1e-10"]["steps_fwd"])
    steps10 = int(a["1e-10"]["steps_fwd"])
    t10 = float(a["1e-10"]["t_reached"])
    lin = m10 + 30 * (m10 - m8)  # linear in -log10 eps: 60 decades = 30 two-decade steps
    geo = m10 * (m10 / m8) ** 30  # geometric: the same ratio per two decades
    print(f"p_max {p}: {m8:.2f} ms (1e-8), {m10:.2f} ms (1e-10); +{(m10 - m8) / 2:.2f} ms per decade, x{m10 / m8:.3f} per two decades")
    print(f"   at 1e-70: linear {lin:.0f} ms per step;  geometric {geo / 1e3:.0f} s per step (x{geo / m10:.2e} the 1e-10 cost)")
    for slab, sf in (("steps x 1 (1e-10's)", 1.0), ("steps x 11.5 (#334's get_eta factor, unverified)", 11.5)):
        for mlab, ms in (("linear", lin), ("geometric", geo)):
            per_unit = ms / 1e3 * steps10 * sf / t10
            out = []
            for wl, wt in (("#334 weights", raw1000 * ratio), ("HM black 1000", w[1000.0] * ratio), ("HM black 1e4", w[1e4] * ratio)):
                out.append(f"{wl} {per_unit * wt / 3600:.2g}")
            print(f"   {slab:48s} {mlab:9s}: {per_unit:9.3g} s per our unit; panel forward core-hours: " + "; ".join(out))

print("\n== p_max = 4 at point (1, 54): local slope of forward wall against 1/eps (MEASURED, three points)")
w8 = float(rd(f"{gg}/below_1e6_eps1e-8.csv")[0]["wall_fwd_s"])
w10 = float(rd(f"{gg}/below_1e6_eps1e-10.csv")[0]["wall_fwd_s"])
w6s = (float([r for r in pr if (r["i"], r["j"]) == ("1", "54")][0]["wall_fwd_s"]),
       float([r for r in rd(f"{gg}/scaling_gg1_lw.csv") if r["Lw"] == "1024"][0]["wall_fwd_s"]))
for w6 in w6s:
    print(f"wall {w6:.2f} s (1e-6, one of its two loads), {w8:.2f} s (1e-8), {w10:.2f} s (1e-10): x{w8 / w6:.2f} then"
          f" x{w10 / w8:.2f}; d log10(wall) / d log10(1/eps) = {math.log10(w8 / w6) / 2:.2f} then {math.log10(w10 / w8) / 2:.2f}")
s8 = int(rd(f"{gg}/below_1e6_eps1e-8.csv")[0]["steps_fwd"])
s10 = int(rd(f"{gg}/below_1e6_eps1e-10.csv")[0]["steps_fwd"])
s6 = int([r for r in pr if (r["i"], r["j"]) == ("1", "54")][0]["steps_fwd"])
print(f"steps {s6}, {s8}, {s10}: x{s8 / s6:.2f} then x{s10 / s8:.2f}")
