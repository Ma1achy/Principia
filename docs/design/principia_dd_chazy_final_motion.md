# Drill-down — Chazy final motion: the classifier, its fields and its stains

*Status: design, ruled by R-442 (the human's rulings of 15–16 October 2026), not yet built. Chazy's (1922)
final-motion classification as a live, per-playhead field, and the stains that display it. A separate system from the
event classes: the event classes say what has **happened** (collision of pair k, escape of body k, bounded at the
horizon, payload §2); this says what kind of **final motion** the system is in, as far as can be told at time t.
Transcribed from the Chazy stain spec, which was written up from prin-rs PR #32; it is not re-derived here.*

---

## 0. Provenance and authority

- **The source.** The spec `chazy_stain_spec.zip` (16 October 2026: `00_README.md`, `01_classifier.md`,
  `02_fields_and_payload.md`, `03_stains_and_palettes.md`, `04_decisions_and_open_questions.md`, `05_tests.md`), given
  by the human with R-442. This drill-down transcribes it. The spec's section numbers are cited as "spec 01 §2" and so
  on, for the record only; this file is what the plan cites.
- **The evidence.** Every measured number here is from prin-rs PR #32 (`Ma1achy/prin-rs`, branch
  `chazy-final-motion`, head `8335db7`): `src/chazy.rs` (the classifier `classify`, the Fewbody reference
  `fewbody_resolved`, Mardling–Aarseth `ma_stable`, the two-body helpers `pair_kup`, `out_kup` and `kepler`),
  `tests/chazy.rs` (the ten fixtures), `examples/chazy_probe.rs` (the Fewbody comparison harness),
  `examples/chazy_map.rs` (the chart maps) and `results/chazy/README.md` (the validation write-up). Where the spec
  leaves something unclear and the PR's code settles it, the code is transcribed here and named by file and function.
- **How the PR is used (the human's follow-up to R-442, a source-handling instruction for this port, not a ruling).**
  1. The PR is a reference, not an authority (R-1, R-159): what the build needs is transcribed here, and the plan
     cites this file, never the PR.
  2. The PR's map colours (HE red, B-stable green, and so on, `results/chazy/README.md` §5) are an early palette and
     are superseded. **Every colour here is the spec's 03**, never the PR's.
  3. The PR ran at `r_coll = 0` and CPU f64. Collisions, f32 parity and the escape-detector alignment are open
     (§9, §11.3) because the PR never exercised them; its silence on them is not a ruling.
- **The previews.** The spec's `img/` strips (each stain on `preset_shape_h1` and `latent_shape` at t = 10, 50, 200,
  800, 48 × 48, Heggie, η = 10⁻², `r_coll = 0`) are not added to the corpus: `docs/design/` holds no images. They are
  regenerated with prin-rs PR #32's `examples/chazy_map.rs` (`cargo run --release --example chazy_map --
  preset_shape_h1 48 results/chazy`, and the same for `latent_shape`), recoloured with §10's palettes; the PR's own
  PNGs use the superseded palette. In Principia the stains' golden images (§10) are the visual record. Because
  `r_coll = 0` integrates through binary collisions, **no collision appears in any preview**; Principia stops a run at
  `r_coll`, and those pixels draw in the collision colour (§7).
- **The follow-up of 16 October 2026 (R-443).** The human added a section to the spec, 03 §10 "One body at a time",
  with additions to 02 (two derived fields), 04 (a ruling, three calibrations and an open question) and 05 (six
  tests). This drill-down transcribes them: §5.2's last two rows and §5.6, §10.11, §11.1.1, §11.2's last three rows
  and §11.2.2, §11.3 and §12.3. The block's measured counts and distances are the human's own prin-rs runs (48 × 48,
  Heggie, η = 10⁻², `r_coll = 0`, t = 800), given in the block; prin-rs PR #32 at `8335db7` holds the functions the
  fields use (`other`, `pair_kup`, `out_kup`), not those runs. Every colour in §10.11 is the block's. The three cautions
  above hold for it unchanged.
- **"Pixel" in the transcribed text is a sample.** The spec and the prototype's maps ran at one sample per pixel
  (48 × 48), so their "pixel" counts and per-pixel properties are per-sample in Principia's terms, where one pixel may
  resolve several samples (`principia_memory_tiers.md` § "1. Taxonomy (locked vocabulary)": SAMPLE against PIXEL). The
  classifier labels each sample and each ensemble copy (§6), and the requirements say "sample"; the transcribed counts
  stand as given. Where a line is about rendered output (a golden image, a screenshot), "pixel" means a pixel.
- **Body names as shown (R-449).** Bodies are 0-based internally (R-22): the fields, the stain parameter `k`, the
  code and every machine-readable dump keep 0, 1, 2, and a dump's header or schema says so (REQ-GEN-036). Everything a
  person reads (legends, labels, preset names, panel titles, and this file's prose about what is shown) calls them
  body 1, 2, 3, so internal body `k` is shown as body `k + 1`, through the one display conversion, TASK-M2-33's
  (REQ-GUI-188; `principia_dd_simstate_payload.md` §2, "What a person reads").

---

## 1. What it is

**The question.** The payload carries an event, not the final motion in Chazy's sense. This system gives the final
motion at every playhead, as an approximation that holds up to the current time, recomputed at every playhead: a
finite march can only say what the motion looks like at `t`, and watching it evolve is the point
(prin-rs PR #32, `src/chazy.rs`, module note).

**The design in one paragraph (spec 00).** The classifier reads the live state plus one running latch, `I_max`, and
nothing after the playhead, so it passes the live-playhead test (§3.1). Its output is derived on the read side, not
stored. Displayed, it splits into two independent questions on two maps: **what** the final motion is (Chazy's seven
classes plus collision, one flat colour each), and **how settled** the answer is (a status map of the classifier's
working states). A third family of stains puts an energy channel in the brightness slot under Replace-L, which
recovers distinctions the flat map folds away.

**Where it sits.** It depends on the event machinery (M3: collision, escape, the terminal latch, the latches) and on
the colour composition backbone (M7: categorical palettes, the categorical filter, Replace-L, compaction, the preset
suite). The classifier logic is validated in prin-rs on CPU at f64; what Principia must settle that the prototype
could not is in §9 and §11.3.

---

## 2. Chazy's classes

Chazy (1922) classifies the final motion of the three-body problem as t → ∞. Each letter names the energy of one
separation:

- **H, hyperbolic**: leaving with energy to spare; the separation grows linearly in t.
- **P, parabolic**: leaving at exactly escape speed; the separation grows as t^(2/3). A knife-edge between leaving and
  returning, of zero measure.
- **E, elliptic**: a bound pair.

| class | meaning | allowed at |
|---|---|---|
| **H** | all three separate hyperbolically | E > 0 |
| **HP** | all three separate; one pair parabolically | E > 0 |
| **P** | all three separate parabolically | E = 0 |
| **HE** | one body escapes hyperbolically, a bound pair remains | any E |
| **PE** | one body escapes parabolically, a bound pair remains | E < 0 |
| **B** | bounded: nobody ever leaves | E < 0 |
| **OS** | oscillatory: unbounded excursions that keep returning (`limsup` of the largest separation infinite, `liminf` finite) | E < 0 |

**The energy certificates:**

- **E > 0 rules out B and OS.** Lagrange–Jacobi, `I'' = 4E − 2V > 4E > 0`, so `I → ∞`. An interacting E > 0 system is
  **transient**, never "bounded so far".
- **E < 0 rules out H, HP and P.** Total dispersal needs non-negative energy.

**Which half the rule enforces.** The E > 0 half is enforced by §3.2's order: transient (step 3) comes before
B-stable, OS-candidate and B-so-far, so an E > 0 state never reads them. The E < 0 half is not enforced: under the
default hierarchy gate, the dispersal step (§3.2 step 1) and the two-stage H (step 2) never test E. That no E < 0
trajectory committed wrongly in prin-rs PR #32 (§4.1) is a measurement, not a guarantee. The review of this port
constructed an E < 0 state that reads H and then P (an equal-mass homothetic equilateral expansion with E = −0.1 U_0,
at 2.9 and 3.2–3.5 times its initial size, turning back at 10 times), and showed that for equal-mass free fall and for
Burrau (3, 4, 5) the dispersal step cannot fire, while the two-stage H is not excluded. Whether the classifier should
enforce the E < 0 half is RQ-325.

---

## 3. The classifier at a playhead

Source: prin-rs PR #32, `src/chazy.rs::classify`, transcribed.

### 3.1 Inputs and definitions

Every playhead, from the live Cartesian state, the masses, the total energy `E`, the initial hyperradius `R_0` and
the running maximum moment of inertia `I_max`. `G = 1`. Transcribed from prin-rs PR #32 (`src/chazy.rs`,
`src/physics/energy.rs`, and how `examples/chazy_probe.rs` calls `classify`):

- **`E`** is the total energy at `t = 0`, `E_0 = K_0 + V_0`, unsoftened (`energy::energy` at `eps2 = 0` on the initial
  state). The read side already carries it.
- **`I`** is the moment of inertia about the centre of mass, `I = Σ_k m_k |r_k − R_com|²` (`energy::inertia`).
- **`R_0`** is the initial hyperradius, `R_0 = √(I_0 / M)`, `M = Σ m_k` (`energy::hyperradius` on the initial state).
- **A pair `(i, j)`'s two-body energies** (`pair_kup`): reduced mass `μ = m_i m_j / (m_i + m_j)`, `Δr = r_j − r_i`,
  `Δv = v_j − v_i`, `d = |Δr|`; kinetic `K = ½ μ |Δv|²`, potential magnitude `U = m_i m_j / d` (positive), and radial
  velocity `v_r = Δr·Δv / d` (receding when `v_r > 0`).
- **The outer body `b`'s two-body energies** (`out_kup`), against the centre of mass of the other two, `(i, j)`:
  `M_p = m_i + m_j`, `R_c = (m_i r_i + m_j r_j)/M_p`, `V_c` likewise; `Δr = r_b − R_c`, `Δv = v_b − V_c`,
  `μ = m_b M_p / (m_b + M_p)`, `K = ½ μ |Δv|²`, `U = m_b M_p / d`, `v_r = Δr·Δv / d`.
- **The parabolic band** (`band`): a ratio `q = K/U` is `+1` (hyperbolic) if `q > 1 + ε`, `−1` (bound) if `q < 1 − ε`,
  and `0` (parabolic) otherwise, `|q − 1| ≤ ε`.
- **The hierarchy ratio of body `b`** is its distance from its pair's centre of mass over its pair's separation,
  `ρ_b = d_out(b) / d_pair(i, j)`. **The body most separated from its pair** is the `b` with the largest `ρ_b`; on a tie
  the lowest body index wins (`classify`'s fold keeps the first strictly greater). Bodies are 0-based (R-22); the
  classifier names no pair id, so prin-rs's pair order `(0,1), (0,2), (1,2)` does not reach any output. Legends and
  labels read the bodies as body 1, 2, 3 (§0, R-449).
- **`I_max`** is the running maximum of `I` (§6).
- **The live-playhead test** (prin-rs PR #32, `src/chazy.rs`, module note): everything the classifier reads is either
  the live state or a running latch, with no re-run from `t = 0` and nothing after `t`.

### 3.2 The rule, in order

The labels, as the spec gives them (spec 01 §2), with `K` and `U` as in §3.1:

| label | condition | committed |
|---|---|---|
| **H** | every pair separation > `h_far · R_0`, every pair receding, every pair `K/U > 1 + ε` | yes |
| **HP**, **P** | as H, with one or two (HP) or all three (P) pairs inside the parabolic band `\|K/U − 1\| ≤ ε` | yes |
| **HE** | the body most separated from its pair has hierarchy ratio > `h_far`, is receding, and has outer `K/U > 1 + ε`; the pair is bound, `K < U` | yes |
| **PE** | as HE with the outer `K/U` inside the band | yes |
| **B-stable** | a bound hierarchy that passes Mardling & Aarseth (2001) | yes |
| **excursion** | as HE but the outer body is bound to its pair (`K/U < 1 − ε`) | no |
| **transient** | none of the above, and `E > 0` | no |
| **OS-candidate** | none of the above, `E < 0`, and `I_max / I > os_ratio` | no |
| **B-so-far** | none of the above, `E < 0` | no |

A two-stage break-up (escaper gone and its former partners unbound to each other) is H.

**The order `classify` evaluates them in** (prin-rs PR #32, `src/chazy.rs::classify`, transcribed; `settled` is §3.6's
settling test, always true with settling off):

1. **Dispersal.** If every pair separation exceeds `h_far · R_0` and `settled` (under the tidal gate, §3.6: if
   `E > 0`), and every pair recedes (`v_r > 0`), take each pair's band. If no pair is bound (no band `−1`), the label is
   **H** when no pair is in the band, **HP** when one or two are, **P** when all three are. Otherwise go on.
2. **Hierarchy.** Take the body `b` most separated from its pair `(i, j)`. The pair's binding is a **sign**: `−1` if
   `K_pair < U_pair`, else the pair's band (`0` or `+1`). The gate is `ρ_b > h_far` and `settled` (under the tidal gate,
   §3.6's tidal test). If the gate passes and the outer body recedes:
   - pair bound and outer band `+1`: **HE**;
   - pair bound and outer band `0`: **PE**;
   - pair bound and outer band `−1`: **excursion**;
   - pair not bound (band `0` or `+1`) and outer band `0` or `+1`: **H**, the two-stage break-up;
   - otherwise (pair not bound, outer bound) go on.
3. **Transient.** If `E > 0`: **transient**.
4. **B-stable.** If Mardling–Aarseth is on, the pair is bound, the outer body is bound to it (`K_out < U_out`) and
   `ma_stable(i, j, b)` holds (§3.5): **B-stable**.
5. **OS-candidate.** If `I_max / I > os_ratio`: **OS-candidate**.
6. Otherwise **B-so-far**.

The spec's table and the code's order differ in two places:

- **B-stable against excursion.** The table lists B-stable before excursion; the code returns excursion inside the
  hierarchy step (step 2), before it reaches B-stable (step 4). A bound, Mardling–Aarseth-stable hierarchy whose outer
  body is past `ρ > h_far` and receding reads excursion under the code and B-stable under the table. This is
  reachable and common: of 23,675 random Mardling–Aarseth-stable bound hierarchies generated in the review of this
  port, 11,066 fall in that branch, and a stable triple would read excursion on the outgoing half of each outer orbit
  and B-stable on the incoming half, flickering on the status map (§10.6). Both fold to B in `chazy_final` (§5.3), so
  the final-motion maps do not change; `chazy_class` and `chazy_status` do. The fixture
  `a_mardling_aarseth_stable_hierarchy_is_b_stable…` (§12.1) passes under both because its outer `v_r` is exactly 0.
  Which order holds is RQ-324.
- **B-stable against transient.** The table lists B-stable before transient; the code tests transient first. They
  differ only for a Mardling–Aarseth-stable hierarchy with `E > 0`, and no such state has been shown to exist: the
  review bounded the outer potential's departure from its monopole at about 0.11 `|E_pair|` at quadrupole order, and
  the largest `E / |E_pair|` found over 400,000 random states and a hill-climb under the B-stable conditions was
  −0.9986. Which order holds is RQ-318.

**When `I_max` is read.** `classify` documents its latch as read before the current playhead updates it, and
`examples/chazy_map.rs` calls it so; `examples/chazy_probe.rs`, the validation, updates `I_max` first. The two give
the same label whenever `os_ratio ≥ 1`: if the current `I` is a new maximum the ratio is at most 1 either way, and
otherwise `I_max` is the same in both.

### 3.3 Defaults

`ChazyCfg::default()` (prin-rs PR #32, `src/chazy.rs`), each value a calibration confirmed at a gate (§11.2):

| parameter | value | meaning |
|---|---|---|
| `gate` | `Hierarchy` | commit on the hierarchy ratio, not tidal isolation |
| `h_far` | 5 | hierarchy ratio, and dispersal distance in units of `R_0` |
| `ε` | 0.05 | half-width of the parabolic band in `K/U` |
| `os_ratio` | 25 | `I_max / I` above which a bound system is an OS-candidate |
| `τ` (settling) | ∞ (off) | closure-chord settling gate; measured as pure delay |
| `δ` | 10⁻⁵ | tidal tolerance, read only by the tidal gate (§3.6) and the reference (§3.7) |
| `mardling_aarseth` | on | enables B-stable |

### 3.4 Two choices that are physics, not tuning

- **A pair's binding is a sign, not a band.** A bound pair at pericentre has `K/U = (1 + e)/2`, inside any parabolic
  band once `e > 0.9`. Testing it against the band misread eccentric binaries as marginal and called the system a
  two-stage H. The pair test is `K < U`. The fixture `an_eccentric_pair_at_pericentre_is_still_bound` (§12.1) holds
  it, and reverting the sign test makes it fail (checked in prin-rs PR #32, `results/chazy/README.md` §1).
- **Parabolic is a band, not a class.** P, HP and PE need an exact zero of a two-body energy and have zero measure. A
  finite run cannot certify them. They are reported so the map can show where the margin is thin, and scored with
  their hyperbolic neighbour, `coarse`: HP, P → H; PE → HE; every other label is itself.

### 3.5 Mardling–Aarseth

Planar: `R_p^out / a_in > 2.8 [(1 + q_out)(1 + e_out)/√(1 − e_out)]^(2/5) (1 − 0.3 i/180°)`, with `i ∈ {0°, 180°}`
(prograde, retrograde). Transcribed from prin-rs PR #32 (`src/chazy.rs::ma_stable` and `kepler`), for an inner pair
`(i, j)` and an outer body `k`:

- **Kepler elements** of a relative orbit `(Δr, Δv)` of total mass `m`: specific energy `ε_K = ½|Δv|² − m/|Δr|`,
  angular momentum `h = Δr_x Δv_y − Δr_y Δv_x`, `a = −m / (2 ε_K)` (negative if unbound),
  `e = √max(0, 1 + 2 ε_K h² / m²)`.
- **Inner orbit:** the pair's relative orbit, total mass `M_b = m_i + m_j`, giving `a_in` and `h_in`.
- **Outer orbit:** body `k` about the pair's centre of mass, total mass `M_b + m_k`, giving `a_out`, `e_out` and
  `h_out`.
- **Not a hierarchy** unless `a_in > 0`, `a_out > 0` and `e_out < 1`; then the test has no answer and B-stable does not
  fire.
- `q_out = m_k / M_b`; `R_p^out = a_out (1 − e_out)`; the inclination factor is 1 when `h_in · h_out ≥ 0` (prograde)
  and 0.7 when not (retrograde, `1 − 0.3`).
- Stable iff `a_out (1 − e_out) / a_in > 2.8 [(1 + q_out)(1 + e_out)/√(1 − e_out)]^0.4 × factor`.

### 3.6 The options the fixtures exercise

The defaults ship one gate and no settling, but the fixtures (§12.1) exercise the other settings, so the port carries
them (prin-rs PR #32, `src/chazy.rs`, `Gate` and `ChazyCfg`):

- **The tidal gate** (`Gate::Tidal`, Fewbody's isolation). Dispersal's distance test becomes `E > 0`. The hierarchy
  test becomes `2 m_b / (m_i + m_j) · (r_pair / d_out)³ < δ`, with `r_pair = a_in (1 + e_in)` when the pair's Kepler
  `a_in > 0` and the pair separation otherwise.
- **Settling** (`τ` finite): the shape-sphere chord over one snapshot, `|n − n_prev| < τ`, where `n_prev` is the
  previous snapshot's shape vector, held in a second latch; with no previous snapshot it is not settled. With `τ = ∞`
  it is always settled and no previous-shape latch is kept.

### 3.7 The Fewbody reference

Fewbody (Fregeau, Cheung, Portegies Zwart & Rasio 2004, MNRAS 352, 1, §3.3.5): resolved iff every top-level pair
recedes, the tidal ratio on every binary is below δ, every tree is Mardling–Aarseth stable, and the top-level energy
is positive. It is the reference the classifier is measured against, not a candidate. Transcribed for three bodies
from prin-rs PR #32 (`src/chazy.rs::fewbody_resolved`):

- Find the pair with positive Kepler `a` and the smallest `a`.
- **No bound pair:** H if every pair recedes and `E > 0`; otherwise unresolved.
- **A bound pair `(i, j)`, third body `k`:** if the outer body is bound to it (`K_out < U_out`), B-stable if
  `ma_stable(i, j, k)` holds, otherwise unresolved. If not bound, HE if the outer body recedes and
  `2 m_k / (m_i + m_j) · (a (1 + e) / d_out)³ < δ`, otherwise unresolved.

Its "unresolved" is the reference's only; the classifier never returns it.

### 3.8 The labels: ten

`classify` returns ten labels: **H, HP, P, HE, PE, B-stable, B-so-far, transient, OS-candidate, excursion**
(prin-rs PR #32, `src/chazy.rs`, `Motion`, less `Unresolved`, which only the reference returns). The spec's
"the nine labels of 01 §2" (spec 02 §2) counts the table's rows, HP and P sharing one; the field has ten. Committed
labels are H, HP, P, HE, PE and B-stable (`Motion::committed`).

---

## 4. Validation

All numbers in this section are prin-rs PR #32's (`results/chazy/README.md` §3–§5, `probe_n600_seed1000.txt`,
`probe_n600_seed50000.txt`, `map_*_n48.txt`).

### 4.1 The probe

**The sample.** 1200 trajectories (two seeds of 600) from random latent ICs: `z_α, z_β ~ N(0, 1.5²)`, `z_μ ~ N(0, 1)`,
momenta `z_q ~ N(0, s²)` with `s` = 0 for half the sample (free fall), 0.3 for a quarter and 1.0 for a quarter.
Heggie, η = 10⁻², `r_coll = 0`, snapshots every 0.4 (the escape rule's closure window). 0 of 1200 failed.

**The truth.** The same trajectory run on in 10-unit chunks until the motion is unambiguously asymptotic (hierarchy
ratio or minimum separation above 10³) or `t = 10⁴`, then classified by Chazy's definition: the growth exponent
`β = d ln r / d ln t` of each pair separation over the second half of the run. All three `β > 0.5` at minimum
separation > 50 is H/HP/P (`β > 0.85` hyperbolic). At hierarchy > 100, a bound pair by its Kepler energy (which the
tidal ratio < 2 × 10⁻⁶ makes rigorous) with an escaper `β > 0.85` is HE, 0.5–0.85 PE. Otherwise unresolved, or
B-stable if Mardling–Aarseth stable at the end. (The first truth tested the pair's own separation exponent, which an
eccentric binary at two arbitrary phases reads as growth; 40 genuine HE trajectories were "unresolved" while every
method committed on all 40. Replaced by the Kepler energy test, the unresolved count fell 49 → 13.)

| | seed 1000 | seed 50000 |
|---|---|---|
| wrong commits on E < 0, any playhead | **0** | **0** |
| first correct label, E < 0, median (default) | **51.6** | **40.8** |
| Fewbody δ = 10⁻⁵ | 69.6 | 69.6 |
| Fewbody δ = 10⁻³ | 52.0 | 42.8 |
| never correct by T = 1000 (default) | 16 | 18 |
| Fewbody δ = 10⁻⁵ | 30 | 27 |
| with the settling gate on (τ = 0.01), median | 86.4 | 77.2 |

The settling gate bought zero precision for 35 time units of delay, so it ships off and the classifier needs no
previous-shape latch. `h_far` = 3 and 5 differ by under 2 points of correct share at every horizon; `h_far = 10` was
run only with settling on, where settling binds and it equals `h_far = 5` (`results/chazy/README.md` §4.1).

### 4.2 Where it is not perfect

- **One temporary wrong commit on an E > 0 system** (seed 1000, T = 10, 1 of 300), withdrawn by T = 20. **One
  temporary commit on an unresolved trajectory** (seed 50000). Fewbody made neither.
- **OS-candidate is not validated.** 2–3 per seed later escaped as HE. It means "has been far out and come back", not
  a prediction of oscillatory motion. OS is believed to have zero measure and no finite run can certify it.
- **B-stable never fired** on random starts. It is covered by a constructed fixture only.
- **PE truth is weak.** A slightly hyperbolic escape still reads as parabolic at t ~ 10³.
- **The sample is a choice.** Half of it is prompt dispersal that every method gets right. A slice through a chaotic
  chart weights the long-lived population more.

### 4.3 On real charts (48 × 48)

`preset_shape_h1` (free fall, E < 0 throughout), the share of the view in each class:

| class | t = 10 | t = 50 | t = 200 | t = 800 |
|---|---|---|---|---|
| HE | 11.89% | 32.99% | 67.14% | 90.49% |
| PE | 0.69% | 0.95% | 0.52% | 0.22% |
| B-so-far | 77.17% | 46.18% | 17.06% | 3.39% |
| OS-candidate | 2.17% | 1.39% | 2.26% | 1.04% |
| excursion | 8.07% | 18.49% | 13.02% | 4.86% |

No H and no transient appear: free fall has E < 0. No transient is enforced by the rule's order (transient needs
E > 0); no H, HP or P is what was observed on every sample, which the rule does not guarantee (§2, RQ-325).
`latent_shape`: 81.25% H and 18.75% HE from t = 50; 2.99% transient at t = 10, resolved by t = 50.

---

## 5. Fields

### 5.1 Derived, not stored

Every quantity the classifier and the stains need is a function of the live state `(r, p)`, the masses, `E_0` and
`R_0`, except one running latch (§6). The read side already carries `E_0` and the live state. So each field below is
a **generated read-side accessor** beside `energy_drift` (`principia_dd_simstate_payload.md` §5, derived at read),
recomputed at every playhead. A palette change re-styles with zero dispatches: it is render key (seam 5,
`principia_systems_architecture.md` §4–§5). The classifier writes nothing to the payload and changes no event class.

### 5.2 The fields

| field | type | values | used by |
|---|---|---|---|
| `chazy_class` | categorical, 10 | the ten labels of §3.8 | everything; the source of the others |
| `chazy_final` | categorical, 8 | B, H, HP, P, HE, PE, OS, collision | the final-motion stains |
| `chazy_status` | categorical, 7 | committed, B-stable, B-so-far, excursion, transient, OS-candidate, collision | the status stain |
| `chazy_structure` | categorical, 4 | dispersal, hierarchy, interacting, oscillatory | the structure × energy stain |
| `chazy_q` | scalar ≥ 0 | `K/\|U\|` of the separating motion; 1 is parabolic | the energy channel |
| `chazy_escaper` | categorical, 3 | body index 0, 1 or 2 (internal; shown as body 1, 2, 3, §0): the body most separated from its pair (§5.6, R-443) | the one-body views (§10.11) |
| `chazy_qk[k]`, k = 0, 1, 2 | scalar ≥ 0, per body | body k's `K/\|U\|` against the centre of mass of the other two; 1 is parabolic (§5.6, R-443) | the body lens with energy (§10.11.2) |

### 5.3 `chazy_final`: the labels folded onto Chazy's seven

| classifier label | final class |
|---|---|
| H, HP, P, HE, PE | itself |
| B-stable, B-so-far | **B** |
| excursion | **B** (bound to its pair) |
| transient | **B**: nobody has left yet at the playhead |
| OS-candidate | **OS** |
| collision-terminated (Principia only) | **collision** |

Folding transient into B is a display choice, not a physics claim (R-442). A transient system has E > 0 and cannot stay
bound, but at the playhead it has not separated. The legend reads B as **"bound, or nobody has left yet"**. The
distinction is recovered by the status map (§10.6), and by any stain with the energy channel in the brightness slot
(§10.5).

### 5.4 `chazy_status`

`committed` for any label the classifier has committed to among H, HP, P, HE and PE. B-stable is committed but shown
in its own colour, because it is the one bound label that is a prediction. The other four (B-so-far, excursion,
transient, OS-candidate) are the working states, and collision passes through.

### 5.5 `chazy_structure` and `chazy_q`

Which bodies are separating from which, and how hard:

| structure | when | `q` |
|---|---|---|
| dispersal | label is H, HP or P | the smallest pair `K/U` (the least unbound pair) |
| hierarchy | the most separated body has hierarchy ratio > `h_far` | the outer body's `K/U` relative to the pair |
| interacting | otherwise | total `K / \|U\|` (> 1 exactly when E > 0) |
| oscillatory | label is OS-candidate | total `K / \|U\|` |

`q` is compacted for display as `b = q / (1 + q)`: bound below ½, parabolic at ½, hyperbolic above. This form needs
no range (`principia_dd_colouring.md` §3.6, the range-free forms, R-411).

Which row holds where several apply is not given by the spec; it is a definition requirement (R-72), written here by
the task that builds the field. The overlaps it settles:

- an OS-candidate, or a two-stage H, whose most separated body has ratio > `h_far`;
- a transient sample whose most separated body has ratio > `h_far` (the hierarchy step's gate passed, but the outer
  body was not receding, or the pair was unbound with the outer body bound, so `classify` fell through to transient);
- a bound label (B-stable, B-so-far, OS-candidate) whose most separated body has ratio > `h_far`, its outer body
  approaching.

Taken literally, the table's "when" column puts these in the hierarchy row, and then `q` is the outer `K/U`. The review
of this port constructed a transient state that this reading draws dark (equal masses; 0-based, as in the fixtures, pair
(0, 1) unbound and body 2 at ρ = 10, receding, outer `K/U = 0.5`; `E = +0.628`, transient by falling through the
hierarchy step; `q = 0.5`, `b = 1/3`, `L = 0.5`, below the parabolic mid-point), against §10.5.1 and R-442's ruling 5;
and, the other way, a B-so-far whose outer body approaches with outer `K/U > 1` draws light. Spec 03 §4.2 (§10.5.2 here)
lists transient only under interacting. So the definition must keep REQ-COL-078's property on these states: a transient
sample above the parabolic mid-point, a sample with a bound label below. One definition that does is to take the
hierarchy row only for the hierarchy step's own labels (HE, PE, excursion), and otherwise the row by label (dispersal
for H, HP and P, oscillatory for OS-candidate, interacting for the rest); the task chooses and writes it (applied per
R-369: the definition can satisfy both §5.5 and ruling 5, so this is not an RQ).

### 5.6 `chazy_escaper` and `chazy_qk` (R-443)

The spec's 02 addition (the human's follow-up of 16 October 2026), transcribed:

| field | type | definition |
|---|---|---|
| `chazy_escaper` | categorical, body index 0–2 | the body most separated from its pair: argmax over `b` of `\|r_b − R_cm(other two)\| / \|r_i − r_j\|` (the same hierarchy-ratio choice `classify` uses for HE/PE) |
| `chazy_qk[k]`, k = 0..2 | scalar ≥ 0, per body | body k's `K/\|U\|` against the centre of mass of the other two (`out_kup(k)` in prin-rs PR #32, `src/chazy.rs`); 1 is parabolic |

- **Derived, nothing stored.** Both come from the live state and the masses, so they are read-side accessors like the
  other Chazy fields (§5.1). Nothing new is stored, and neither needs a latch.
- **The masses are on the read side already.** The spec says "Masses are already in the payload". They are the
  `ICDescriptor`'s `m0 m1 m2`, written once post-decode (`principia_render_contract.md` § "Part 1 — The payload
  (render input)"; R-86), which the read side holds as `ctx.ic` (the same contract, § "Part 4 — Semantic rules", the
  generated accessors). A derived field names them in its `from`, as `n` and `energy_drift` do
  (`principia_dd_generation_root.md` §3.8). Where masses vary across a slice, each sample reads its own.
- **`chazy_escaper`** is §3.1's body most separated from its pair: the `b` with the largest hierarchy ratio
  `ρ_b = d_out(b) / d_pair(i, j)`, where `(i, j)` is the pair not containing `b` (prin-rs PR #32, `src/chazy.rs::other`:
  0 → (1, 2), 1 → (0, 2), 2 → (0, 1)), `d_out(b)` is `out_kup(b)`'s distance and `d_pair` `pair_kup(i, j)`'s.
  Bodies are 0-based (R-22). **Ties:** the block does not say what two equally separated bodies give; `classify`
  settles it, as §3.1 transcribes: its fold runs over `b = 0, 1, 2` from `(0, 0)` and keeps the first strictly greater
  ratio, so on a tie the lowest body index wins (prin-rs PR #32, `src/chazy.rs::classify`). Legends and labels read
  the index as body 1, 2, 3 (§0, R-449; REQ-GUI-188); a machine-readable export keeps it 0-based and says so
  (REQ-GEN-036). The field is defined at every sample; the one-body views read it only on HE and PE samples (§10.11).
- **`chazy_qk[k]`** is the outer `K/U` of `out_kup(k)` (§3.1, with `b = k`): `M_p = m_i + m_j` for the other two,
  `μ = m_k M_p / (m_k + M_p)`, `K = ½ μ |Δv|²`, `U = m_k M_p / d`. It is ≥ 0, below 1 bound, 1 parabolic, above 1
  hyperbolic. On a HE or PE sample, `chazy_qk[chazy_escaper]` is the outer ratio the classifier banded, which is also
  `chazy_q`'s hierarchy row (§5.5).
- **Which body the event names.** The event stain's escape detail names a body by Principia's own escape rule
  (payload §2; R-29). It and `chazy_escaper` can name different bodies on the same pixel at the same playhead, the same
  gap as §9 item 3 (RQ-313). Which one the one-body views read is RQ-321.

---

## 6. The latch: `I_max`

`I_max`, the running maximum of the moment of inertia (§3.1), mirrors `d_min` (payload §1–§2): it is forward-only, so
it is live-compatible. It starts at `I_0` (prin-rs PR #32, `Latch::new(i0)`). With settling off there is no
previous-shape latch.

- **Cadence.** The prototype updated it once per snapshot (every 0.4). With settling off, cadence matters only for
  this latch, and it is **updated at every step, not every snapshot** (spec 02 §6.4). The measured `os_ratio` (§11.2)
  was taken at the snapshot cadence.
- **Per copy.** Each ensemble copy carries its own `I_max`: the class is a point quantity, classified per copy and
  anti-aliased by the SSAA resolve (`principia_dd_colouring.md` §3.7).
- **Storage (proposed, open).** A log-quantised value fits the reserved descriptor bits 10–14 (32 half-octave levels
  up to 2¹⁶), leaving bit 15 for the open departure flag (R-37's per-sample "departed" bit, whose placement is open:
  REQ-VAL-055). Payload §2 holds bits 10–15 "Reserved, decode as zero, never opportunistically reused", so spending
  them is a versioned format change, as `last_symbol`'s bits 8–9 were, and needs a ruling: RQ-310.

---

## 7. Validity and styles

- **Running samples hold a value.** The class is a label as of t, so it never draws the not-yet stipple. This is
  colour_composition §4.1's case of a field that holds a value for a `running` sample, rendered in every mode.
- **`sim_failed` or NaN** draws `#141418`, B's black, as every presentation view draws a failure and a non-finite
  value (R-467); View › Debug view shows it, `debug_invalid` or the NaN / Inf style. *Was (R-136):* "draws the invalid
  hatch, as everywhere (render contract, presentation layer; R-136)".
- **Collision-terminated** has no final motion. It reads `collision` in `chazy_final` and `chazy_status`, never a
  motion class, and draws in the collision colour (§10.1) in every stain: a pair collision, a triple collision and a
  collision at `t = 0` alike.
- **Escape-terminated**: the production loop stops at escape fire, so the class freezes at the fire-time state. It can
  then read excursion or B where the event stain reads escape (§9, RQ-313).
- **A pixel the chart hatches** (`principia_chart_decoder_contract.md` Part 3: the types, `validate`'s layers 2 and 3,
  a failed `Φ`) is not part of the chart: it draws the plain canvas background in every Chazy stain, with no key, decided
  from the chart alone, never from the payload's `decode_failed` detail (Part 3; R-446, R-467). Only the debug view
  hatches it.
- **The decoder's own `decode_failed`** samples (its `DEGENERATE` tags and a non-finite decode) have no state to
  classify, and draw `#141418` in every Chazy stain, as the outcome view draws them (R-446) and as `sim_failed` draws
  here: the human's ruling R-467 names the Chazy stains among the presentation views, where a failure "keeps R-446's
  black #141418" (ruling RQ-319, option (b)). That black is B (§10.1), "bound, or nobody has left yet", so in the
  figure a failure reads as B, as the outcome view's failures read as its bounded / nothing yet; a free-fall slice under
  `η_E` reads black in both systems. A reserved `state` code draws `#141418` too. Every sample with no motion to
  classify thus draws B's black, and a pixel the chart hatches the background; View › Debug view tells them apart, each
  under its own key with a count (R-467 D5). *Was (RQ-319, decided per R-369, option (a)):* "draw the invalid hatch,
  `debug_invalid`, in every stain, as `sim_failed` does"; "Every sample with no motion to classify thus draws the
  hatch".

---

## 8. Parity

The class flips on the sign of a two-body energy and on band edges in `K/U`. The spec calls it **Tier N**
(`principia_parity_contract.md` §2): CPU and GPU agree to tolerance, not bit for bit. A pixel within ulps of a sign
change may differ between backends. **The parity test measures that disagreement rate rather than asserting zero.**
How a categorical label fits the parity contract's tiers, and the f32 measurement, are RQ-314.

---

## 9. Gaps between the prototype and Principia

The prototype is CPU f64 and differs from Principia in four ways. None changes the design:

1. **Collisions.** The prototype regularised through binary collisions (`r_coll = 0`). Principia terminates at
   `r_coll`, so collision is a class here (§7) and was never exercised there.
2. **f32.** The prototype was f64 only. The sign tests near `K = U` need a parity measurement at f32 (§8, RQ-314).
3. **The escape detector.** Principia's escape event uses its own rule (`principia_01_pitfalls.md` §2.2; R-29) and
   stops the run at fire. The classifier's HE gate (hierarchy ratio > 5, receding, outer `K/U > 1 + ε`) is not the same
   test. A pixel can read "escape" in the event stain and excursion or B here at the same playhead. Whether that is
   acceptable (they answer different questions) or one should gate the other is RQ-313.
4. **Cadence.** The prototype classified every 0.4 time units. With settling off, cadence only matters for the
   `I_max` latch, which is updated at every step (§6).

---

## 10. Stains and palettes

Each stain is a composition on the existing backbone (`principia_colour_composition.md` §4): a categorical palette in
the colour slot, optionally a field in the brightness slot under Replace-L, and optionally posts. Nothing here is a
new pixel function. Every palette is a default, user-editable like any other swatch set (colour_composition §1.4).
**Hue carries the class (R-442):** the default final-motion stain has no lightness encoding, so lightness stays free for
another field, and a stain that uses lightness uses it for a named field (energy) in the brightness slot.

### 10.1 The two rules that hold in every stain

- **Collision is magenta `#DA85B6`.** A run that ended in a collision has no final motion. One colour for every
  collision, pair or triple: which pair collided is the event stain's job (R-442).
- **B is black `#141418`**, the same swatch as the event palette's "bounded" (colour_composition §1.4), and B-stable
  and B-so-far are one class in the final-motion maps (R-442). The spec has it with no hue (`a = b = 0` in OKLab), so
  that under Replace-L it becomes neutral grey at the brightness field's lightness, still the one class with no hue.
  `#141418` is not quite neutral (OKLab `a ≈ 0.002`, `b ≈ −0.008`, chroma ≈ 0.008): RQ-317.

The not-yet stipple never appears (§7). `sim_failed`, NaN and `decode_failed` draw B's `#141418`, as in every
presentation view (R-467; §7); the hatch is the debug view's. *Was:* "The invalid hatch draws `sim_failed` and NaN as
everywhere".

### 10.2 Final motion: Chazy's seven (the default)

Field `chazy_final`. Colour slot only, brightness None. **The default final-motion stain (R-442).** The chromatic
classes share one OKLab lightness (L = 0.72, chroma 0.12), so hue alone carries the class and lightness stays free for
another field.

| class | colour | sRGB |
|---|---|---|
| B | black | `#141418` |
| H | cyan | `#14BBC2` |
| HP | blue | `#55AEE8` |
| P | violet | `#A497EA` |
| HE | coral | `#E4896A` |
| PE | amber | `#CC9C42` |
| OS | green | `#7BB76E` |
| collision | magenta | `#DA85B6` |

The logic: everything flying apart is cool, stepping cyan → blue → violet as it gets more parabolic; one body
escaping is warm, coral for hyperbolic and amber for parabolic; OS green; bound black.

Legend: "Final motion (as of t = …)". B is labelled **"bound, or nobody has left yet"** (R-442).

### 10.3 Final motion: four families

Field `chazy_final` through `coarse` (§3.4). The quickest read.

| family | classes | sRGB |
|---|---|---|
| dispersal | H, HP, P | `#14BBC2` |
| escape | HE, PE | `#E4896A` |
| bound | B | `#141418` |
| oscillatory | OS | `#7BB76E` |
| collision | | `#DA85B6` |

### 10.4 Final motion: Okabe–Ito

The seven on the Okabe–Ito set, designed to stay distinguishable under the common colour-vision deficiencies.
Collision stays magenta. That costs one pair: Okabe–Ito's reddish purple P `#CC79A7` sits 0.041 from collision's
magenta `#DA85B6`, unsimulated and under each simulation, the variant's tightest pair (§11.2.1). Without collision the
seven clear 0.170 unsimulated and at least 0.079 simulated.

| B | H | HP | P | HE | PE | OS |
|---|---|---|---|---|---|---|
| `#000000` | `#56B4E9` | `#0072B2` | `#CC79A7` | `#E69F00` | `#F0E442` | `#009E73` |

### 10.5 With an energy channel

Both stains here put `chazy_q` in the brightness slot under Replace-L. Lightness `L = 0.3 + 0.6 · b` with
`b = q/(1 + q)`: dark is bound, mid (L = 0.6) is parabolic, light is hyperbolic. This is Replace-L's range form
(`principia_dd_colouring.md` §3.5, R-77) at `L_min = 0.3`, `L_max = 0.9`. The floor of 0.3 keeps dark regions' hue
legible; lowering it to near 0 makes bound regions black at the cost of their hue (a calibration, §11.2).

#### 10.5.1 Seven classes × energy

The seven-class hue with energy as lightness: how deep inside its class each pixel sits. Because B has no hue, its
lightness is the energy alone. **This recovers what the flat map folds into B**: a transient system (E > 0) renders
light grey and a genuinely bound one dark grey.

#### 10.5.2 Structure × energy

Field `chazy_structure` in the colour slot. Hue says which bodies are separating from which; lightness says how hard.

| hue | sRGB | light | mid | dark |
|---|---|---|---|---|
| dispersal | `#14BBC2` | H | HP / P | not reachable |
| hierarchy | `#E4896A` | HE | PE | bound hierarchy (B, excursion) |
| interacting | `#A497EA` | transient (E > 0) | | bound, still mixing (B) |
| oscillatory | `#7BB76E` | | | OS |

Chazy's letters are this same energy reading, so every class and the classifier's working states fall out as hue plus
lightness, with no extra colours. P and HP share a cell: one scalar cannot separate them, and P has zero measure.

### 10.6 Status

Field `chazy_status`. **A separate map, so the final-motion palettes need only Chazy's seven (R-442).** Read it beside
a final-motion map: that one says what, this one says how sure.

| status | sRGB | meaning |
|---|---|---|
| committed | `#E2E3E8` | a final class the classifier has committed to (H, HP, P, HE, PE) |
| B-stable | `#2F6DB5` | bound, and passes Mardling–Aarseth: predicted to stay bound |
| B-so-far | `#160928` | bound, nothing has escaped, no stability guarantee |
| excursion | `#E08A2E` | bound, one body far out but still bound to the pair |
| transient | `#16A394` | E > 0 and still interacting: must disperse, not yet how |
| OS-candidate | `#7BB76E` | has been far out and come back |
| collision | `#DA85B6` | |

On `preset_shape_h1` the pale grey grows from 13% at t = 10 to 91% at t = 800; B-so-far drains from 77% to 3%;
excursions peak at 18.5% at t = 50. On `latent_shape` it is 97% committed at t = 10 (prin-rs PR #32,
`results/chazy/map_*_n48.txt`).

### 10.7 Filter

The categorical filter (colour_composition §1.4) applies unchanged: show a set of classes, mute the rest. "Just
escapes" (HE, PE) is the example. The spec's previews mute to `#3A3B40`; the mute transform is the one
REQ-COL-058 defines in colour_composition §1.4 for every categorical mode, so the Chazy stains use it and the preview
value is not a second rule (applied per R-369; R-442).

### 10.8 Time views

These summarise the history of the label, so they need more than the current state.

#### 10.8.1 When it was decided

One image for a whole run. Colour is the final class; lightness is how early it settled for good (palest by t = 10,
darkest at t = 800, grey never). On `preset_shape_h1`: 280 pixels settled by t = 10, 484 more by 50, 788 by 200, 538 by
800 and 214 never (the spec's preview run of prin-rs PR #32's `examples/chazy_map.rs`).

**Live implementation:** a second forward-only latch, the time the current coarse class was first committed, reset
whenever it changes. Same shape as `I_max`, and it needs its own storage bits (RQ-310). Without that latch this view
is offline only.

#### 10.8.2 What just changed

White where the coarse class changed since the previous playhead, the rest dimmed. It needs the previous frame's
class: a render-side comparison between frames, not a payload field. Where that comparison lives, given that a stain
node reads the payload at the playhead (render contract Part 2), is RQ-320.

### 10.9 Greyscale and hue modes

REQ-COL-067 and REQ-COL-068 need a rank order for any categorical field (colour_composition §4.1). For `chazy_final`,
bound first and darkest, as the outcome class is ranked: **B, OS, PE, HE, P, HP, H, collision**, `K = 8`. The ranks of
`chazy_class`, `chazy_status` and `chazy_structure` are not given by the spec: a definition requirement (R-72),
written here by the task that builds the stains. `chazy_escaper` (§5.6, categorical, 3 values) needs one too, written
here by the task that builds the one-body views. The one-body views' roles (§10.11.1) are a stain's output, not a field:
greyscale and hue mode render a field (colour_composition §4.1), so the roles take no rank (applied per R-369).

### 10.10 Legends

Generated from the stain (`principia_render_gui_spec.md` §G6), with shares of the view. The title always carries the
playhead: "Final motion (as of t = …)". The status stain's legend names the working states in full. OS-candidate and
B-stable are labels, not predictions; whether the legend help says they are provisional is RQ-315. B's label, "bound,
or nobody has left yet", holds in every final-motion legend (the seven classes, the four families' bound, Okabe–Ito).
A stain with the energy channel (§10.5, §10.11.2) has §G6's separate key for what brightness encodes: a bar for `q`
(or `q_k`) through `b = q/(1 + q)`, parabolic marked at ½, beside the hue swatches (applied per R-369, §G6's rule).
Bodies are named 1, 2, 3 in every legend (§0).

### 10.11 One body at a time (R-443)

Spec 03 §10, the human's follow-up of 16 October 2026, transcribed. Which body escapes is never drawn in the
final-motion maps (the event stain carries escape of body k). These views follow one body, so the detail stays
legible. Collision is magenta `#DA85B6` and bound is black `#141418` throughout, as everywhere else (§10.1). Like every
stain in §10, each is a composition on the backbone, its palette a user-editable default; §7's validity rules hold in
each (running samples hold a value; `sim_failed`, NaN and `decode_failed` `#141418`, a pixel the chart hatches the
canvas background, R-467, ruling RQ-319; *was:* "`sim_failed` and NaN the hatch; `decode_failed` the hatch too"). The fields are §5.6's.

#### 10.11.1 Body lens

A chosen body `k` (a stain parameter, 0, 1 or 2 internally, shown and chosen as body 1, 2 or 3; R-443, §0). Each
sample shows body k's role:

| role | rule | sRGB |
|---|---|---|
| escaping, hyperbolic | class HE and `chazy_escaper == k` | `#E4896A` |
| escaping, parabolic | class PE and `chazy_escaper == k` | `#CC9C42` |
| in the pair left behind | class HE or PE and `chazy_escaper != k` | `#4158BD` |
| flying apart with the others | class H, HP or P | `#8A8A90` |
| bound | B-stable, B-so-far, excursion, transient, OS-candidate (the B and OS folds of 02 §2, here §5.3) | `#141418` |
| collision | run ended in a collision | `#DA85B6` |

The two escaping colours are the seven-class palette's HE and PE (§10.2), so the meaning carries across.

- **OS-candidate draws bound black here, not OS green.** In `chazy_final` OS-candidate folds to OS, `#7BB76E` (§5.3,
  §10.2); the lens's bound row takes both folds, B's and OS's, so OS-candidate is black. That is the block's plain
  reading ("the B and OS folds of 02 §2"), and no corpus rule contradicts it: the lens asks what body k is doing, and in
  an OS-candidate no body has left (applied per R-369, recorded, not an RQ).
- **"Class"** is `chazy_class`'s label at the playhead (§3.8); the role table is exhaustive over its ten labels plus
  collision, so a new label breaks the build until it is mapped (§12.3 item 1).
- **The escaping body is `chazy_escaper`** as the block gives it. Whether the views should read the event's escape
  detail instead, where the run has an escape event, is RQ-321.
- **The bound row's legend label is "bound, or nobody has left yet"**, R-442's label for B, because the row folds in
  transient (E > 0, nobody has left) and OS-candidate as `chazy_final`'s B does. A plain "bound" would carry the
  misreading R-442's label avoids (applied per R-369; the same in the escaper map, §10.11.4).

#### 10.11.2 Body lens with energy

The §10.11.1 roles in the colour slot, `chazy_qk[k]` in the brightness slot under Replace-L, compacted
`b = q/(1 + q)`, `L = 0.3 + 0.6 b` (dark bound, mid parabolic, light hyperbolic), the same mapping as §10.5 (REQ-COL-088,
REQ-COL-089). Grey and black have no hue, so under Replace-L they become neutral greys at body k's energy: inside the
bound regions this shows how close body k has come to leaving before anything commits.

`#141418`'s chroma is ≈ 0.008 (RQ-317), and the grey `#8A8A90`'s is ≈ 0.009 (OKLab `a ≈ 0.002`, `b ≈ −0.009`, measured
here with `principia_dd_colouring.md` §3.1's transforms), so neither is exactly neutral: RQ-322.

**"Flying apart" and "bound" draw alike at equal `q_k`.** Both roles lose their own lightness under Replace-L and keep
almost no hue, so a grey-role sample and a black-role sample with the same `q_k` render the same colour. In practice
they separate by energy (flying apart has body k unbound, `q_k` mostly above 1; bound mostly below), but not by
construction. The legend therefore does not show two neutral swatches that cannot be told apart: its role key shows the
§10.11.1 flat colours, its brightness bar shows `q_k` (§10.10), and it says that under the energy channel the two
neutral roles are told apart by the flat lens beside it (applied per R-369; render GUI spec §G6's separate keys).

#### 10.11.3 All three bodies

The §10.11.1 lens for k = 0, 1, 2 as three small panels side by side, at one playhead, titled body 1, 2, 3 (§0). A
layout over §10.11.1, not a new stain. The GUI spec has no view of one figure as several panels in v1; its "side by
side" is a v2 research tool (`principia_render_gui_spec.md` § "G11. Research — first pass (v2)", R-106). Where the
panels live is RQ-323.

#### 10.11.4 Escaper map

Escapes only. Hue is which body escaped; parabolic is the darker shade of the same hue (OKLab L 0.72 hyperbolic, 0.52
parabolic, chroma 0.12, hues 35°, 150°, 265°).

| | body 1 | body 2 | body 3 |
|---|---|---|---|
| HE (escapes hyperbolically) | `#E58870` | `#69BA7C` | `#80A3F0` |
| PE (escapes parabolically) | `#A24B36` | `#287C42` | `#4665AE` |

The columns are `chazy_escaper` = 0, 1, 2, shown as body 1, 2, 3 (§0). H, HP, P draw grey `#8A8A90`; bound black
`#141418`; collision magenta `#DA85B6`. "Bound" is §10.11.1's bound row, the B and OS folds, so OS-candidate is black
here too, and its legend label is "bound, or nobody has left yet" (applied per R-369, the same reading as §10.11.1).
The escaping body is `chazy_escaper` (RQ-321). Re-measured here, the six swatches sit at L 0.72 / 0.52 (±0.002),
chroma 0.12 (±0.001) and hues 35°, 150°, 265° (±0.3°), as stated.

#### 10.11.5 By mass instead of by label

Where masses vary across a slice, "body 1" is not the same physical body everywhere, so rank by mass instead. Two
stains:

- **Escaper by mass rank:** §10.11.4 with hue = the escaper's mass rank (lightest `#E58870`, middle `#69BA7C`, heaviest
  `#80A3F0`) instead of its label.
- **Lightest-body lens (and heaviest-body lens):** §10.11.1 with k = the body of that rank at each pixel.

The block names only the hyperbolic shades by rank. "§10.11.4 with hue = the escaper's mass rank" re-indexes §10.11.4's
columns by rank, so the parabolic shades follow plainly (applied per R-369, transcribed, not an RQ):

| | lightest | middle | heaviest |
|---|---|---|---|
| HE | `#E58870` | `#69BA7C` | `#80A3F0` |
| PE | `#A24B36` | `#287C42` | `#4665AE` |

**Ties.** Where two masses are equal to within a relative 10⁻⁶, the rank is undefined. Such pixels fall back to the
label view, and the legend says so. On an equal-mass chart (all the presets) the mass-rank views are entirely the label
views; the legend says that rather than draw a misleading rank. The block's test (§12.3 item 4) adds that a chart with
one tied pair "falls back only where the tie matters": so a body's rank is undefined where its own mass ties another's,
and a pixel falls back where the rank it needs is undefined (the escaper's, for the escaper map; the lightest's or the
heaviest's, for that lens). Two things are not given, and are a definition requirement (R-72), written here by the task
that builds the views: the label body `k` a rank lens falls back to (a rank lens has no label parameter), and the form
of the relative comparison (relative to which of the two masses). The tolerance's value is a calibration (§11.2).

Because the rank hues are the label hues (lightest = body 1's coral, and so on), a fallback pixel and a ranked pixel
of the same colour mean different things; the legend's fallback flag is what tells them apart.

#### 10.11.6 Measured, for the record

The human's prin-rs runs (48 × 48, Heggie, η = 10⁻², `r_coll = 0`, t = 800), as the block gives them:

- `preset_shape_h1` (equal masses): the escapes split evenly by body, 711 / 708 / 671 pixels. The escaper map shows each
  arm of the tripod as a different body escaping, a symmetry no other view shows. (2090 of 2304 pixels, 90.71%, §4.3's
  HE and PE shares at t = 800.)
- `latent_mass` (masses vary 0.0009 to 0.97 across the slice): 102 escapes; by label 33 body 1 and 69 body 3, two
  regions in two colours. By mass rank: 98 lightest, 4 middle, 0 heaviest; the two regions are one colour, because
  both are the lightest body leaving.
- `latent_shape` (unequal but constant masses): all 432 escapes are body 3, its lightest body. (432 of 2304 is §4.3's
  18.75% HE.)

---

## 11. Decisions, calibrations and open questions

### 11.1 Decided (R-442)

The human's rulings (Malachy, 15–16 October 2026), recorded as R-442:

1. The Chazy final-motion class is a **separate system** from the event classes, with its own stains.
2. **Hue carries the class.** The default final-motion stain uses no lightness, so lightness stays free for another
   field; stains that use lightness use it for a named field in the brightness slot.
3. **B is black `#141418`**, and B-stable and B-so-far are **one class** in the final-motion maps.
4. **Collision is magenta `#DA85B6`**, one colour for every collision, pair or triple.
5. **Transient draws as B** in the final-motion maps, labelled "bound, or nobody has left yet". The energy-channel
   stains and the status map recover it.
6. The working states (B-stable, B-so-far, excursion, transient, OS-candidate) live on **their own status map**, not in
   the final-motion palette.
7. The seven-class palette of §10.2 is the default.

#### 11.1.1 Decided (R-443)

The human's ruling (Malachy, 16 October 2026), recorded as R-443: the one-body views of §10.11 (spec 03 §10) are added
as stains; the lens body is a stain parameter; mass rank is offered alongside label, with the tie fallback of
§10.11.5.

### 11.2 Calibrations to confirm at a gate (R-71)

| calibration | proposed | evidence |
|---|---|---|
| `ε`, parabolic band | 0.05 | not swept |
| `h_far`, hierarchy ratio | 5 | 3, 5 and 10 tried; 3 and 5 differ by under 2 points |
| `os_ratio` | 25 | not swept; OS-candidate unvalidated |
| settling `τ` | off | measured: zero precision for 35 time units of delay |
| `I_max` quantisation | log, half-octave, 5 bits | not measured |
| the palettes of §10 | as listed | measured, §11.2.1 |
| energy lightness range | L 0.3 to 0.9 | chosen so dark regions keep their hue |
| `q` compaction | `q/(1 + q)` | range-free; parabolic at ½ |
| the time views (§10.8): "when decided" lightness by time, "what just changed" dimming | palest by t = 10 to darkest at t = 800, grey never; the dimming not given | the spec's preview run (§10.8.1); where the comparison lives is RQ-320 |
| the body-lens palette (§10.11.1) | as listed | measured, §11.2.2 |
| the escaper palette, by label and by rank (§10.11.4–§10.11.5) | as listed | measured, §11.2.2 |
| the mass tie tolerance (§10.11.5) | relative 10⁻⁶ | not measured |

The evidence column is prin-rs PR #32's (`results/chazy/README.md` §4) and the spec's measurement below.
The last three rows are the human's follow-up (R-443), measured as §11.2.2 gives.

#### 11.2.1 Palette separation, measured

Minimum OKLab distance, against τ = 0.101, the OKLab distance from `#FF00FF` to `#E034C6` that R-132 ruled a collision
and R-429 confirmed (REQ-COL-055, REQ-COL-061). "Styles" are the invalid hatch (`#9B00FF`, `#48FFFF`), the not-yet grey
`#4E4E4E` and the stipple dot `#909090`. CVD simulations use the matrices of `principia_dd_colouring.md` §3.8.

| palette | simulation | within palette (tightest pair) | against styles (tightest) |
|---|---|---|---|
| seven-class | none | **0.083** (H / HP) | 0.137 (P / stipple dot) |
| seven-class | deuteranopia | **0.002** (HP / P) | 0.074 |
| seven-class | protanopia | 0.046 (HP / P) | 0.057 |
| seven-class | tritanopia | 0.014 (H / HP) | 0.062 |
| status | none | 0.114 (transient / OS-candidate) | 0.111 (transient / stipple dot) |
| status | deuteranopia | 0.070 | 0.018 (transient / stipple dot) |
| status | protanopia | 0.065 | 0.034 |
| status | tritanopia | 0.035 | 0.085 |
| four families | none | 0.110 (escape / collision) | 0.137 (oscillatory / stipple dot) |
| four families | deuteranopia | 0.015 (escape / oscillatory) | 0.074 |
| four families | protanopia | 0.086 | 0.057 |
| four families | tritanopia | 0.032 | 0.085 |
| Okabe–Ito | none | **0.041** (P / collision) | 0.121 (P / stipple dot) |
| Okabe–Ito | deuteranopia | 0.041 (P / collision) | 0.035 |
| Okabe–Ito | protanopia | 0.041 (P / collision) | 0.050 |
| Okabe–Ito | tritanopia | 0.041 (P / collision) | 0.094 |
| structure hues | none | 0.110 (hierarchy / collision) | 0.137 (interacting / stipple dot) |
| structure hues | deuteranopia | 0.015 (hierarchy / oscillatory) | 0.074 |
| structure hues | protanopia | 0.077 | 0.057 |
| structure hues | tritanopia | 0.027 | 0.062 |

For comparison, the event-class palette reads 0.175 unsimulated and 0.047 / 0.039 / 0.070 under the three
simulations.

The rows from "four families" down were measured in the review of this port with `principia_dd_colouring.md` §3.1's
transforms and §3.8's matrices (the method reproduces every seven-class and status figure above); the spec gives no
figures for them. Each palette includes collision `#DA85B6`, and the four families B `#141418`, as the stains draw
them; the structure hues are measured flat, as their swatches. The "against styles" column is the distance to the
nearest style colour, the stipple's dot among them, which is spec 04 §2.1's method. R-434's criterion for the stipple
(colour_composition §1.4, REQ-COL-064) is the distance to the farthest pixel of a 4 × 4 block, a stricter test for the
stipple to fail: every Chazy entry is at least 0.179 from its farthest pixel.

**The seven-class palette is under τ by design.** H, HP and P are steps of one cool family, so H and HP sit 0.083
apart, and under deuteranopia HP and P collapse to the same colour. That is the cost of the family logic. The
Okabe–Ito variant (§10.4) is the colour-vision-safe alternative for the seven, at the cost of P against collision
(0.041 in every simulation and unsimulated). Whether the default should change is a decision for the gate (RQ-311).
τ is the corpus's collision criterion between the invalid style and a palette entry (REQ-COL-055, REQ-COL-061) and
between the stipple and a flat colour (REQ-COL-064); no corpus rule holds a palette's own entries τ apart, so the
default stands under R-442 with the shortfall recorded and measured (§12.2, item 7).

**The status palette clears τ unsimulated against everything.** Its B-so-far was moved from slate `#4A4F5C`, which sat
0.023 from the not-yet grey, to near-black violet `#160928`, which clears every entry and style by at least 0.249 in all
four simulations. The weak CVD distances against the stipple dot do not arise in practice, because the Chazy stains
never draw the stipple (§7).

#### 11.2.2 One-body palettes, measured (R-443)

The block's measurement, minimum OKLab distance against τ = 0.101, unsimulated (it gives no simulated values):

- **Body-lens palette:** 0.084, escaping hyperbolic against escaping parabolic (the same coral and amber as HE and PE,
  kept deliberately so the meaning carries). Every other pair, and every entry against the hatch and stipple, clears τ;
  "in the pair" `#4158BD` clears all by at least 0.177. It replaced an earlier steel blue `#6C8EBF` that sat 0.076 from
  the grey.
- **Escaper palette:** 0.102 at its closest (body 1 hyperbolic against collision).
- **Mass tie tolerance:** relative 10⁻⁶.

What those numbers mean against the corpus's τ, as in §11.2.1:

- **The body lens's 0.084 is within the palette.** τ is the collision criterion between a palette entry and the
  invalid hatch (REQ-COL-055, REQ-COL-061) and between the stipple and a flat colour (REQ-COL-064); no corpus rule holds
  a palette's own entries τ apart. The pair is §10.2's HE / PE pair (0.084 there too), kept by design, and the test
  asserts it as measured so a change shows (§12.3 item 6).
- **The escaper palette clears τ against collision by 0.001** (0.102 against 0.101). That is the margin, recorded; a
  later change to either swatch can cross it, and the test asserts the value.
- **Re-measured here** with `principia_dd_colouring.md` §3.1's transforms: the block's 0.084, 0.177, 0.076 and 0.102
  reproduce. The grey `#8A8A90`, in both palettes, sits **0.020** from the stipple dot `#909090`, spec 04 §2.1's
  method (the distance to the dot colour). By the corpus's criterion, R-434's (colour_composition §1.4, REQ-COL-064),
  no flat colour may come within τ of every pixel of a 4 × 4 block of the stipple, which is the distance to the block's
  farthest pixel: for `#8A8A90` that is the not-yet grey `#4E4E4E`, at **0.212**, above τ. So by R-434 the grey clears
  the stipple, and the block's claim that every entry clears the hatch and stipple holds; the 0.020 is recorded as the
  dot distance. Every other entry clears the hatch's two colours, the not-yet grey and the stipple dot by either
  method (the next closest dot distance is 0.137, coral against the stipple dot). The grey's chroma (§10.11.2) is
  RQ-322.

### 11.3 Open questions

1. **Descriptor bits** (RQ-310). `I_max` in reserved bits 10–14 claims space next to the open departure flag (bit 15).
   The "when decided" view (§10.8.1) needs a second latch and more bits. Is that budget available, and is it worth it?
2. **Escape detector alignment** (RQ-313). A pixel can read "escape" in the event stain while the Chazy class reads
   excursion or B at the same playhead (§9 item 3). Acceptable, or should one gate the other?
3. **f32 parity** (RQ-314). Measure the disagreement rate at f32 near the `K = U` sign changes before setting a
   tolerance.
4. **REQ-GUI-076** (RQ-312) bans "Chazy subtitles" from the dev GUI. That is a different feature from a Chazy stain;
   the spec asks for a ruling so the ban is not read as covering this.
5. **OS-candidate and B-stable** (RQ-315) are labels, not predictions: OS-candidate is unvalidated and B-stable has
   never fired on random starts. The spec proposes shipping them as provisional and saying so in the legend help.
6. **Where it lands** (RQ-316). It needs the event machinery (M3) and the composition backbone (M7).

And from this port: the default palette against τ and Okabe–Ito (RQ-311), B's chroma (RQ-317), the order of
B-stable and transient (RQ-318), `decode_failed` (RQ-319, decided per R-369: the hatch) and where "what just
changed" lives (RQ-320). From its review: the order of B-stable and excursion (RQ-324) and whether the classifier
enforces the E < 0 certificate (RQ-325).

And from the follow-up (R-443): the escaper definition, the block's own open question, whether the one-body views read
the event's escape detail when the run has an escape event or `chazy_escaper` always (RQ-321, beside RQ-313); the
grey `#8A8A90`'s chroma (RQ-322); and where §10.11.3's three panels live (RQ-323).

### 11.4 Not done, and natural next steps

- **Standish (1972)**: rigorous sufficient conditions for return, a true certificate for bounded motion that this
  classifier does not implement.
- **Re-run the acceptance on a chaotic chart** inside Principia's own pipeline (`preset_shape_h1`), where the
  long-lived population is heavier than in the random sample.

---

## 12. Tests

### 12.1 The fixtures to port

From prin-rs PR #32, `tests/chazy.rs`. Each fixture is a constructed state with the control arm that says it
exercises the branch it is named for. Masses are equal thirds, `G = M = 1`; each state is shifted to the
centre-of-mass frame; each is classified with `R_0 = 1`, a fresh latch (`I_max = I`) and the defaults, unless the row
says otherwise. Transcribed from the file:

- **`binary_plus_escaper(a, e, d, q_out)`.** Bodies 0 and 1 on a Kepler orbit of semimajor axis `a` and eccentricity
  `e`, at pericentre: `r_p = a(1 − e)`, `v_p = √(M_b (1 + e)/r_p)` (vis-viva), `M_b = m_0 + m_1`, `f_0 = m_1/M_b`,
  `f_1 = m_0/M_b`; positions `(0, −f_0 r_p)` and `(0, f_1 r_p)`, velocities `(f_0 v_p, 0)` and `(−f_1 v_p, 0)`. Body 2 at
  `(d, 0)`, receding radially at `v_out = √(2 q_out (m_2 M_b / d) / μ_out)`, `μ_out = m_2 M_b / (m_2 + M_b)`, so its
  outer `K/U = q_out`.
- **The equilateral directions** `(1, 0)`, `(−½, √3/2)`, `(−½, −√3/2)`.

| test | state | asserts | control arm |
|---|---|---|---|
| `a_bound_pair_and_a_hyperbolic_escaper_is_he` | `binary_plus_escaper(1, 0, 50, 2)` | HE | E < 0 overall |
| `the_parabolic_band_and_the_bound_side_of_it` | `binary_plus_escaper(1, 0, 50, q)`, q = 1 and 0.5 | PE at outer `K/U = 1`; excursion at 0.5; `coarse` maps PE → HE, HP and P → H, excursion → itself | |
| `an_eccentric_pair_at_pericentre_is_still_bound` | `binary_plus_escaper(1, 0.95, 50, 2)` | HE with `e = 0.95` | the pair sits inside the band, `\|K/U − 1\| < ε`, and `K < U`. Reverting the sign test makes this fail |
| `an_escaper_leaving_an_unbound_pair_is_a_two_stage_h` | bodies at `(0, ∓0.5)` moving `(0, ∓½ v_rel)` with pair `K/U = 2`; body 2 at `(50, 0)` receding with outer `K/U = 2` | H | |
| `three_bodies_flying_apart_is_h` | positions 50 × and velocities 2 × the equilateral directions | H, and Fewbody (δ = 10⁻⁵) agrees | E > 0 |
| `an_interacting_positive_energy_system_is_transient_never_bounded` | positions 0.577 × and velocities 2 × the equilateral directions | transient | E > 0; the same configuration at rest is E < 0 and B-so-far |
| `the_os_candidate_needs_the_latch_and_nothing_else` | positions 0.577 × the equilateral directions, at rest | B-so-far with a fresh latch, OS-candidate with `I_max = 2 · os_ratio · I` | |
| `a_mardling_aarseth_stable_hierarchy_is_b_stable_and_only_with_the_test_on` | circular inner pair `a_in = 0.1` (as `binary_plus_escaper`'s pair at `e = 0`, `v_in = √(M_b/a_in)`); body 2 at `(10, 0)` moving `(0, √(1/10))`, prograde | B-stable; B-so-far with the test off; Fewbody agrees | `ma_stable(0, 1, 2)` returns true |
| `the_reference_resolves_he_only_once_tidally_isolated` | `binary_plus_escaper(1, 0, d, 2)`, d = 50 and 20 | Fewbody HE at d = 50, unresolved at d = 20; the tidal gate follows it (HE at 50, not HE at 20); the hierarchy gate commits on both | tidal ratios 8 × 10⁻⁶ and 1.25 × 10⁻⁴ |
| `the_settling_gate_withholds_until_the_shape_has_stopped` | `binary_plus_escaper(1, 0, 50, 2)`, `τ = 10⁻³`, `n_prev` = `n` + (0.1, 0, 0) and `n` | withheld when the shape moved, HE when still | |

### 12.2 New, for Principia

1. **Burrau smoke test.** The Burrau ICs to a horizon past their escape: the class ends HE, and E < 0 holds the
   certificate (no H, HP, P at any playhead). The production loop stops at escape fire (R-103) and the class freezes
   there (§7), where it can read excursion or B (RQ-313), so the test runs in the validation harness's post-escape
   march, where R-103 places the pitfalls §2.4 checks. Burrau is a close-encounter benchmark, so it runs at the pinned
   `r_coll` and records it (integrator contract Part 7; REQ-VAL-049); at a nonzero `r_coll` the run can end in
   collision instead, which the test reports as a failure, not a pass (applied per R-369). Whether the E < 0 half is
   enforced is RQ-325.
2. **Collision.** A run terminated at `r_coll` reads `collision` in `chazy_final` and `chazy_status`, never a motion
   class.
3. **The fold table** of §5.3: every classifier label maps to the stated final class and status. Transient → B in
   `chazy_final` and → transient in `chazy_status`. Exhaustive over the enum, so a new label breaks the build until it
   is mapped.
4. **Energy certificates, per pixel** (per sample, §0). On a free-fall chart (E < 0 everywhere) no pixel reads H, HP,
   P or transient at any playhead. On an E > 0 chart no pixel reads B-so-far, B-stable or OS-candidate. The E > 0 half
   and "no transient" hold by the rule's order; "no H, HP or P" is empirical (§2, RQ-325).
5. **B has no hue.** The B swatch has OKLab a = b = 0 to within quantisation, so Replace-L gives neutral grey.
   Control: a chromatic swatch fails the same check. (`#141418`'s chroma: RQ-317.)
6. **Energy channel recovers transient.** Under §10.5.1, a transient pixel's lightness is above the parabolic
   mid-point and a bound pixel's below. Control: the flat map draws both identically.
7. **Palette separation.** Measured the way REQ-COL-055 measures, against R-132's τ = 0.101. The status palette and
   the four-family palette must clear τ unsimulated, within themselves and against the hatch and stipple. The
   seven-class palette is known to fail within itself (H / HP 0.083, §11.2.1): the test asserts the measured values so
   a change is visible, and asserts every entry clears the styles. The Okabe–Ito variant (P / collision 0.041) and the
   structure hues are measured the same way and asserted as measured (§11.2.1's rows from the review).
8. **f32 parity.** The fraction of pixels whose class differs between f32 and f64 on a gallery chart, reported, with
   the differing pixels checked to lie within the band of a sign change.
9. **Live-playhead property.** The classifier reads nothing after t: classifying a run at playhead t, then continuing
   the run, never changes the label already given at t.

### 12.3 One body at a time (R-443)

The block's 05 additions, transcribed:

1. **Role table, exhaustive.** For every class label and every (k, escaper) pair the §10.11.1 role is as tabled. A
   new label breaks the build until mapped.
2. **Exactly one escaping body.** On every HE/PE pixel, exactly one of the three lenses reads "escaping" and the other
   two read "in the pair". On every other pixel, none reads "escaping".
3. **Label symmetry.** Relabelling the bodies of an IC permutes `chazy_escaper` and the lens panels the same way.
   Control: a fixture where the escaper is not body 0, so the test is not passing on a default. Its fixtures keep the
   largest hierarchy ratio strictly largest, since an exact tie goes to the lowest index (§5.6) and does not permute
   (applied per R-369).
4. **Mass rank.** On a fixture with distinct masses the rank views agree with the label views composed with the rank
   permutation. On an equal-mass fixture they fall back to the label views exactly, and the legend flag is set.
   Control: a fixture with one tied pair falls back only where the tie matters.
5. **Energy lens recovers the escape.** For body k escaping hyperbolically, `chazy_qk[k] > 1` and the pixel's lightness
   is above the parabolic mid-point; for a bound k below. Control: the flat lens draws two bound pixels with different
   `q_k` identically.
6. **Palette separation** as measured in §11.2.2, asserted to the measured values so a change is visible; the grey
   `#8A8A90` against the stipple by R-434's criterion (0.212 from the block's farthest pixel, above τ) as well as its
   dot distance (0.020).

---

## 13. References

- J. Chazy (1922), Ann. Sci. École Norm. Sup. 39, 29: the final-motion classes.
- J. M. Fregeau, P. Cheung, S. F. Portegies Zwart, F. A. Rasio (2004), MNRAS 352, 1, §3.3 (arXiv:astro-ph/0401004):
  Fewbody's classification and termination test.
- R. A. Mardling & S. J. Aarseth (2001), MNRAS 321, 398: hierarchical triple stability.
- E. M. Standish (1972), Celest. Mech. 6, 352: rigorous sufficient conditions for return; the certificate this
  classifier does not implement.
- prin-rs PR #32 (`Ma1achy/prin-rs`, branch `chazy-final-motion`, head `8335db7`): the reference implementation and
  the evidence for every measured number above.
