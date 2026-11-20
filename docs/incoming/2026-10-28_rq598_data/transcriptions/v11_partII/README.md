# V-11, what the repo's own paper says about hierarchical stability: Part II, read from its page images

A record, not authority (R-1, R-159). Target V-11 (`../../../2026-10-27_papers/TARGETS.md`, "V-11. Hierarchical stability
limits") says the number to compare comes from the primary, Mylläri et al. 2018 (Part I, *MNRAS* 476, 830), which is not
committed (below) and was not read for this file. The one hierarchical-stability paper that is in the repo is Part II,
Pasechnik, Mylläri, Valtonen & Mikkola 2023, *MNRAS* 525, 1929 (CC BY 4.0;
`../../papers/pasechnik_myllari_valtonen_mikkola_2023_MNRAS_525_1929.pdf`, sha256
`76eab52c4473695e311bb836a53cea1936d53e3d09eb4cc7ba965a573e6662c7`). Part II restates the set-up and the stability
definition Part I uses and gives a closed-form estimate. This file transcribes those, from the page images (the text layer
garbles the formulas), with the page of each. It adds nothing from Part I and does not pretend to.

## Part I

The owner's copy of Part I is held locally, gitignored, at
`docs/incoming/_local/myllari_valtonen_pasechnik_mikkola_2018_MNRAS_476_830.pdf` (`../../README.md` § 1). It was not read for this file.
**V-11 proceeds from Part I in a later local task**, which reads that copy; this file is only what Part II gives.

## Definitions (p. 1929–1931)

- **Set-up** (p. 1929–1930). Two bodies on an inner orbit and a third on an orbit about their centre of mass. Fifteen
  parameters; scaled to ten. "We fix the binary total mass to two mass units and specify m₁, the heavier binary mass, and m₃,
  the third body mass." The binary semi-major axis is one unit, a_out is measured in it. Free elements: m₁, m₃, a_out, ι (the
  relative inclination), e_out, e_in; the less important ones are t₀ (the initial phase of the binary), ω_in, ω_out and Ω (the
  longitude of the ascending node of the outer orbit). The initial binary plane is the fundamental plane.
- **Q.** "The perihelion distance Q = a_out (1 − e_out)." The mean motion of the binary n is unity by definition.
- **Q_max.** "Define Q_max as the largest possible Q that leads to an instability within a given interval of time", as a
  function of m₁, m₃, e_in, e_out and ι, with t₀, ω_in, ω_out and Ω taking any values.
- **Instability** (p. 1929): "the instability defined by either of these two outcomes", the third body no longer bound to the
  binary, or the exchange of one inner member with the third body. Collision is a third outcome, used only in Fig. 8
  (p. 1933: e_in = 0.99 "as leading to a collision"). The interval is "10 000 outer revolutions of the third body" (p. 1929).
- **Protocol** (p. 1930). The first method: start with Q high enough to be stable for N = 10 000 revolutions, lower it in steps
  of 0.1 until instability. The method used: start at Q = 1 ("a guaranteed unstable configuration"), raise Q by 0.1 "until
  10 stable configurations in a row were found. The first stable value of Q in this series was taken as the Q_max." It is
  repeated for 100 values of cos ι and 50 values of Ω; a Q_max(cos ι, Ω) surface is drawn for each e_in. "We found that the
  result is not integrator dependent" (a symplectic method, Runge–Kutta–Fehlberg RK4(5), Hermite, and a three-body
  regularisation code; p. 1930).
- **The experiments' parameters** (Figs 1, 3–6, 13): m₁ = 1.5 M☉, m₂ = m₃ = 0.5 M☉, a_in = 1 au, e_out = 0.5, e_in = 0.55, 0.6, 0.65
  (Figs 3–5), ι up to 140°–154° and Ω = 50°–130° in the finer scan (Fig. 6, cos ι step 0.002, Ω step 1.6°).

## The closed-form estimate (p. 1931–1932; the equations are unnumbered in Part II)

With K = Q^1.5 · √(2(m₁+m₂)/(m₁+m₂+m₃)), the single-encounter energy change of the binary for a parabolic outer orbit
(Roy & Haddow 2003; leading Bessel terms) is, as printed on p. 1931:

    δε/ε ≃ 12 · m₃/(m₁+m₂) · (√π/4) · { Q⁻³ K^2.5 e^(−(2/3)K) } · e_in
           × { [ (1 − 4/9 e_in²) cos ι + 0.5 √(1 − e_in²) (1 − 5/36 e_in²) ] (1 + cos ι²) cos(2ω_out + n t₀) sin dΩ
             + [ √(1 − e_in²) (1 − 5/36 e_in²) cos ι + 0.5 (1 − 4/9 e_in²) ] (1 + cos ι²) sin(2ω_out + n t₀) cos 2Ω
               + 1/3 (1 − 1/8 e_in²) (1 − cos ι²) sin(2ω_out + n t₀) }.

(The "sin dΩ" is as printed. Whether it reads sin 2Ω, and whether "cos ι²" means cos² ι, cannot be settled from the page image;
check against the print.) The quantity in the first curly brackets is "well approximated by a factor proportional to (Q/Q₁)⁻⁷",
with Q₁ = 2.5 (1 + m₃/(m₁+m₂))^(1/3) (Valtonen & Karttunen 2006). Keeping the constructive-interference terms:

    δε/ε ≈ 0.1 · m₃/(m₁+m₂) · (Q/Q₁)⁻⁷ · (1 − 0.5 e_in²) · [1 + cos ι]² · { sin(2ω_out + n t₀ ± 2Ω) }

(− for retrograde orbits, + for direct). By energy conservation δε/ε ≈ −m₃/(m₁+m₂) · (1 − e_out)/(Q/Q₁) · δE/E. After N encounters
the energy changes by √N δE, and setting that to E gives (p. 1932):

    Q ≈ 3.7 (√N)^(1/6) · (1 + m₃/(m₁+m₂))^(1/3) · (1 − 0.5 e_in²)^(1/6) · [1 + cos ι]^(1/3) · (1 − e_out)^(−1/6).

The paper's own caveat: "the inclination factor must be more complex than [1 + cos ι]^(1/3) as Q cannot go to zero at cos ι = −1."
Fig. 7 (e_in = 0.6; Q from 2.6 to 4.4 over cos ι from −1 to 1) shows this single-encounter-plus-random-walk curve dashed, a curve
from Mardling's theory (private communication) dotted, and the numerical Q_max values as crosses "well above either line". **The
printed formula, evaluated with the Fig. 7 parameters, does not reproduce the dashed curve's end values**; see
`check_formula.py`. Which N, and which form of the inclination factor, Fig. 7 used is not stated.

**The printed coefficient does not follow from Part II's own equations.** Take the two equations above,
δε/ε ≈ 0.1 (m₃/(m₁+m₂)) (Q/Q₁)⁻⁷ (1 − 0.5 e_in²) [1 + cos ι]² and δε/ε ≈ −(m₃/(m₁+m₂)) (1 − e_out)/(Q/Q₁) · δE/E, with
Q₁ = 2.5 (1 + m₃/(m₁+m₂))^(1/3), and set √N |δE/E| = 1. That gives

    Q ≈ 2.5 · 0.1^(1/6) · (√N)^(1/6) · (1 + m₃/(m₁+m₂))^(1/3) · (1 − 0.5 e_in²)^(1/6) · [1 + cos ι]^(1/3) · (1 − e_out)^(−1/6),

a coefficient of 2.5 × 0.1^(1/6) = **1.70**, not 3.7. The printed 3.7 is 2.5 × 10^(1/6) = 3.67, as if the 0.1 were 10. Both forms also give
**Q = 0 at cos ι = −1**, which the paper itself concedes cannot be right. `check_formula.py` prints both coefficients and Q with each. So the
closed form, in either coefficient, is **not** V-11's comparison number; that number is Part I's.

## What the numerical results say (qualitative)

- Q_max depends on Ω as cos 2Ω, "in a doubly periodic manner" (Figs 3–5, p. 1931).
- Near ι ≈ 140° (cos ι ≈ −3/4) there is a narrow resonance where Q_max is "about twice as large as at surrounding values of cos ι"
  (p. 1930), attributed to the octupole Kozai–Lidov resonance (§4, p. 1932–1933), with quadrupole and octupole terms comparable.
- With a collision defined as e_in reaching 0.99, Q_max is not raised for e_in < 0.6 and Q_max → ∞ as e_in → 0.99 (Fig. 8, p. 1933).
- Q_max "typically increases going from retrograde orbits to direct orbits" (p. 1932).

## What V-11 can use today, and what it cannot

The V-11 test (`TARGETS.md`: scan the outer pericentre for prograde and retrograde outer orbits on a hierarchical slice) can take
its **definitions** from here: Q, Q_max, the stability interval (10,000 outer revolutions), the instability outcomes (escape or
exchange), the scan protocol and the parameter set above. It cannot take Part I's general Q_max(e_in, e_out, ι, m₃) from here, nor Part II's
closed form (above): the number the target is to compare against comes from Part I, read from the local copy in a later task. The book values on the target's list
(Harrington 3.5 semi-major axes, Valtonen & Karttunen 4.8 and 2.6, Bohlin 4.7, Innanen 70 % and 144 % of the Hill radius) are
secondary and unchanged.
