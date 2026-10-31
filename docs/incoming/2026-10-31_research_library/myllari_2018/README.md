# V-11: what Part I gives (Mylläri, Valtonen, Pasechnik & Mikkola 2018, MNRAS 476, 830, doi:10.1093/mnras/sty237)

A record, not authority (R-1, R-159). Target V-11 is at `../../2026-10-27_papers/TARGETS.md`; Part II's reading is at
`../../2026-10-28_rq598_data/transcriptions/v11_partII/README.md`. The paper is local only
(`../../_local/myllari_valtonen_pasechnik_mikkola_2018_MNRAS_476_830.pdf`, sha256
`625a5494b0709c2bda4fae11e6dbacf6bfb6c63b6e0caa13c1cbcb653f63b55c`). No Principia code ran.

**Part I prints no numerical table.** R-594's "transcribed twice" therefore has only equations and stated values to act
on. The 5445 points are shown only as figures (Figs 1–9 and Appendix A), and the data aren't given.

## Readings

Reading A is by eye from page images. Reading B is Apple Vision OCR of the same equations plus the text layer: both
fragment the mathematics, so they confirm the constants below and not the layout. The third check is numeric (below).

| Item, page | Read by eye | Second reading |
|---|---|---|
| Eq. (11), p. 833 | f(e_in, cos i) = {1 − ⅔ e_in [1 − ½ e_in²] − 0.3 cos i [1 − ½ e_in + 2 cos i (1 − (5/2) e_in^(3/2) − cos i)]} | OCR and text: "− 0.3 cos i", "+2 cos i", "− cos i", exponent "5 3/2" agree; fractions fragmented |
| Eq. (12), p. 833 | g(m1, m2, m3) = (1 + m3/(m1 + m2)) | agrees |
| Eq. (13), p. 833 | Q_st = A (λ√N/(1 − e_out))^(1/6) (f g)^(1/3) | agrees in the text layer; OCR garbled |
| Eq. (9), p. 833 | Q ≈ (λ√N)^(1/6) M^(−1/6) (1 + m3/(m1+m2))^(1/3) (1 − e_out)^(−1/6) (5/3 + cos i)^(1/3) | agrees in part |
| Eq. (8), p. 833 | M = 0.4 (m1+m2)^(7/3) / ((m1+m2+m3)^(1/3) m1 m2); "M^(−1/6) is set to unity in the following" | agrees |
| A values, pp. 834–836 | 1.40 fit; 1.45 ± 0.3 mixed layer; 1.15, 1.45, 1.75; mean 1.6, σ 0.26 over 5445 points; 2.4 | OCR agrees |
| Final form, p. 836 | Q_st = 10^(1/3) A [(f g)²/(1 − e_out)]^(1/6); "A ~ 1 … in N-body simulations, and A = 2.4 in tests for absolute stability" | text agrees |

`formula_check.txt` (`../scripts/myllari_formula.py`) evaluates the equations as read. At Fig. 5's parameters and cos i =
+1 they give 3.147, 3.968, 4.789, and the figure's curves end there at about 3.2, 4.0, 4.75 (by eye), which supports the
reading of eq. (11).

## Definitions

- **Q** (p. 831): "the ratio of the initial pericentre distance of the outer orbit to the inner semimajor axis".
- **Q_st** (pp. 830–831): "the smallest Q value that allows the system to be stable over N = 10 000 revolutions of the
  initial outer orbit". Stepping Q down, "we use the Q-value first encountered in the search for the unstable system to define
  Q_st", even if a lower Q is stable again. (Part II's Q_max is the largest Q that does go unstable.)
- **Instability** (p. 830): "either the escape of one of the bodies, or the exchange of the members between the inner and
  outer systems".
- **Set-up** (p. 831). m1 = 1.5, m2 = 0.5, m3 = 0.5, a_in = 1, e_out = 0.5 as standard. e_in in {0, 0.05, 0.1, 0.15, 0.2,
  0.3, 0.4, 0.5, 0.6, 0.75, 0.9, 0.95, 0.99}. 50 (or 100) cos i values from −1 to +1. Q-step 0.25, later about 0.1. The
  pericentres, mean anomalies and nodes: "Unless otherwise stated below, their values are set to zero."
- **The universality grid** (p. 835). e_in five values, uniform on 0 to 0.99; Ω and cos i 11 values each; m2 ∈ {0.1, 0.5, 1.0},
  m3 ∈ {0.1, 1.0, 10}, m1 = 2 − m2; e_out = 0.5; 5445 points. A: "mean of A = 1.6 and the standard deviation σ = 0.26"
  (the distribution is Fig. 6).
- **Integrators** (p. 831). A symplectic Bulirsch–Stoer (Mikkola 1997), checked on a smaller set with Aarseth & Zare 1974's
  regularisation: "the overall result is not integrator dependent".
- **Exception** (p. 834): "The limit A = 1.75 is good everywhere except at the resonance around i = 140°."

## What V-11 compares

REQ-VAL-330 asks for the limits as distributions. Prograde is cos i = +1 and retrograde cos i = −1, on the same slice.
The comparison is the **upper envelope of first instability against eq. (13)**, as A: on the grid above, the distribution
of A against mean 1.6, σ 0.26, and A ≤ 2.4 outside the 140° resonance. Part I is explicit that the fit isn't exact (p. 833)
and that the scatter is a layer, not a surface (p. 831).

- Part II's closed form remains **not** the number (its own record, above).
- At e_in = 0 the printed f makes the two directions' ratio (0.7/0.1)^(1/3) = 1.913. The book's 4.8/2.6 = 1.846 gives no
  masses or e_out, so this is shape only, not a comparison.
- Part I's introduction gives Harrington 1972 as "at least 3.5 for direct orbits and 2.75 for retrograde orbits". TARGETS.md's
  V-11 lists Harrington's 3.5 only. Secondary; TARGETS.md isn't changed here.
- REQ-VAL-331's tolerance remains a calibration, never set from these A values to make it pass.

## Recorded, not corrected

- **Queued, RQ-639.** "At the limit of e_in = i = m3 = 0, f g = 1" (abstract, p. 830; p. 836). Eq. (11) gives f(0, cos i = 1)
  = 0.7, and f = 1 at cos i = 0, that is i = 90°.
- Fig. 7's caption says "e_in = e_out = 0" and Fig. 8's "e_out = 0.3". Their drawn curves look the same. Eq. (13) at e_out = 0.3
  gives 2.37, 2.99, 3.61 at cos i = +1, close to both figures (about 2.35, 3.0, 3.6 by eye), and at e_out = 0 gives 2.24, 2.82,
  3.40. Fig. 7's curves may be drawn for 0.3. By eye; not a finding about the data.
