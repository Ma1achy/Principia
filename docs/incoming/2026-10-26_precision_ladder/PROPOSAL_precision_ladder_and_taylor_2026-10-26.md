# Principia: the precision ladder and the Taylor stepper

**Draft design note, 26 Oct 2026. Not authority.** It proposes changes to rulings in force (R-265, R-572, canonical spec
§1 item 3, R-90). Nothing here is built until the human rules on §6. Its parts: what exists, the gap, the design, a
build order, tests, the rulings it touches with open questions, and sources. It has no "Decided" section yet, because
nothing is decided. An independent review of the first draft was applied; §8 lists what it changed.

The human's words that started it:

> "does the high order taylor method fight Principia's impl? can we do something similar optionally? (maybe as part of
> the eventual double & double double and arbitary precision stuff)"

> "surely we can do it on GPU also, if we do the whole emulating double precision etc"

> "yeah we could make this whole part of the research quality dump mode, but maybe so it's faster we could emulate like
> double and double double precision on GPU to make it faster? (what about arbitrary on GPU?? if not we just do
> arbitrary precision on CPU)"

> "can you do Heggie or different regularisation with taylor ? and yeah you don't necessarily have to say with
> quad-float use Taylor but you can. again if you really want to you can use taylor on f32 or double float. again
> customisable fully. make this stuff pair well with how everything else already works."

---

## 1. What exists

- **Two occupant axes already.** An occupant is stepper × regularisation (integrator contract Part 2, Part 2b). The
  steppers are KDK, Yoshida-4, Yoshida-6 and RK4 (Euler for debug only). The regularisations are `none`,
  Aarseth–Zare, Heggie 1974 and logH. Every pair runs; a pair measured not to work carries a warning and is never
  blocked (R-561). The default is Heggie × RK4 (R-560). Each occupant advertises a capability profile
  `{order, force_evals, symplectic, step_reversible, run_reversible}` (the reversibility split is R-500), and
  downstream features read the profile rather than assuming (Part 2).
- **Precision is fixed per device.** The kernel is generic over `Real`, instantiated for f32 and f64 only (R-265). The
  GPU is "f32-locked" (canonical spec §1 item 3). Double-double (`Dd`) is parked: "it returns only if a precision
  question needs it and Brutus is too slow" (R-265).
- **Arbitrary precision exists only as a checker.** R-33's Brutus-style CPU reference (TASK-M3-25) raises precision and
  tightens tolerance until the answer stops changing. It checks; it does not produce images.
- **The Result.** R-572's full-precision batch run: one view's region, a uniform grid, refinement off, written to a
  folder that reopens as a view. Its precision choice is "f64 (default, CPU) or f32 (GPU). Nothing else."
- **Philosophy §7.1 already records the idea and why it waits.** Horizon `t_max = ln(1/eps)/λ` is about 23 at f32,
  52 at f64 and 105 at double-double. Szebehely & Peters' Burrau disruption sits at t ≈ 60, "past both current paths.
  The instrument cannot presently reach its own system's most famous published result."

## 2. The gap

- An **image** of a Result cannot run past t ≈ 52 and still be true: philosophy §7.1's horizons use a slice-averaged
  λ ≈ 0.7, so they describe a region, not one trajectory. The single published Burrau trajectory is not the gap: R-33's
  reference can integrate one trajectory to t ≈ 60, and regularised double-precision codes reproduce it (Wang,
  Nitadori & Makino 2020 integrate it to t = 63). The gap is **the neighbourhood of Burrau, mapped pixel by pixel**
  past t ≈ 52, where a Brutus-style reference per pixel is too slow. That is R-265's own condition for unparking: "a
  precision question needs it **and Brutus is too slow**".
- There is no fast extended-precision path: R-33's reference is the only one above f64, and it is a checker.
- At high precision, the existing steppers waste the digits. RK4's error goes as h⁴: at ~28 digits it needs steps
  around 1e-7, which is hopeless. A precision tier without a stepper whose order can rise with the digits buys nothing.
- The GPU, where the throughput is, is barred from extended precision by "f32-locked", although emulated
  double-float on f32 hardware is a known technique and philosophy §7.1 already names it.

## 3. The design

### 3.1 Precision becomes a third occupant axis

An occupant becomes **stepper × regularisation × precision**, each chosen independently, as Part 2b made
regularisation independent of the stepper. Every combination runs; combinations measured to be poor carry a warning,
never a block (R-561, extended).

| Precision | Where it runs | Significant digits | Rough cost vs f32 (estimate, to measure) | Notes |
|---|---|---|---|---|
| `f32` | GPU, CPU | ~7 | 1× | the survey kernel; unchanged |
| `df32` (double-float, 2×f32) | GPU, CPU | ~14 | 10–20× | exponent range stays f32's; for hardware without f64 (all Apple GPUs) |
| `f64` | CPU (and GPU with native f64) | ~16 | — | the shared kernel; unchanged |
| `qf32` (quad-float, 4×f32) | GPU, CPU | ~28 | 50–150× | the GPU's ceiling |
| `dd64` (double-double, 2×f64) | CPU (and GPU with native f64) | ~32 | ~10–20× f64 | the parked `Dd` of R-265, unparked |
| `mp(bits)` (MPFR) | CPU only | any | grows with bits | rayon over pixels |

Why the GPU stops at `qf32`: arbitrary precision on a GPU needs a width fixed at compile time; each extra limb costs
registers and arithmetic; and the payoff over many CPU cores shrinks as the width grows. CUDA libraries do it (CAMPARY,
CUMP), but not for runtime-chosen precision, and not portably through rust-gpu. Past ~30 digits, `mp(bits)` on the
CPU. Li & Liao's runs at hundreds of digits are CPU work too.

Native f64 on the GPU exists through wgpu's `SHADER_F64` feature on Vulkan and DX12, not on Metal or WebGPU. On
NVIDIA consumer cards it runs at 1/32–1/64 of
the f32 rate (AMD RDNA2 about 1/16; Intel Arc has none), so `df32` may beat it (philosophy §7.1 says the same). Both are
offered where the hardware allows; the pilot (§3.6) times them.

**How it fits the code.** Each precision is one more type implementing `Real`. The payload stays `PayloadReal` (f32 and f64; §3.6). The kernel
is already generic, so the physics is the same source at every precision, as R-265 wanted for f32 and f64. The
rust-gpu build gains `df32` and `qf32` as newtypes over f32; `dd64` and `mp` are CPU-only types.

### 3.2 Taylor joins the stepper axis

A new stepper, **Taylor(order)**, with `order` a number or `auto`. It works with every regularisation and every
precision, including f32 and `df32`: nothing ties it to the high tiers, though it is the stepper that makes them pay.

**How it works.** The state's Taylor series in the integration time are built order by order by automatic
differentiation: each operation in the equations of motion (add, multiply, divide, square root, power) has a
recurrence that gives the k-th coefficient of its result from the lower coefficients of its inputs. A product is a
Cauchy product, `(fg)_k = Σ_{j≤k} f_j g_{k−j}`. Order n costs O(n²) per step but takes long steps, and the order can
rise with the digits. Jorba & Zou's rule chooses order ≈ −½ ln(eps) + 1 and the step from the last two coefficients;
the citation and exact form are to be transcribed and checked.

**It must hold integrator contract Part 2c's rules, and as first drafted it did not.**
- *Rule 1, no precision-dependent constant in a control-flow decision.* `Taylor(auto)` choosing its order from
  `eps(P)` inside the kernel would make two instantiations run different logic. So the order is a **fixed integer field
  of the sim key**. `auto` is resolved on the host, once, into that integer, and the sidecar records it.
- *Rule 2, comparison-only branches.* Jorba & Zou's step `(ε/‖x_p‖)^{1/p}` is a runtime power. It sizes the step (a
  value) and may not decide a branch. Its thresholds are frozen per sim key on the host (R-502), and whatever feeds a
  branch is built under R-547.
- *Rule 3, every loop bounded.* The Newton landing (§3.4) has a fixed iteration cap, and each iteration is counted.

**Profile.** `order` is the chosen n (or the order `auto` resolves to); `symplectic` false; `step_reversible` measured
by dd test 3 as for every regularised occupant (R-500), expected ✗; `force_evals` does not apply, so the profile
reports a measured equivalent cost per step (§6, open question 4).

### 3.3 Taylor with each regularisation

Yes, all four. Each regularisation's equations of motion are built from elementary operations, so each has recurrences.
How well it suits Taylor varies:

- **Heggie: the best fit.** As transcribed in the prin-rs reference (not authority), Heggie's
  `Γ* = (H − h) R₁R₂R₃` is a polynomial: "degree six in the coordinates and two in the momenta", with "no `1/r` term
  anywhere". Its Hamilton's equations are therefore polynomial too, so under Eq. 20 (`dτ = dt/(R₁R₂R₃)`) every
  recurrence is a Cauchy product. The default time transformation, Eq. 22 at n = 3/2 (R-161), scales the equations by
  `S^{−3/2}` and adds a term in `Γ*/S`. That costs one power recurrence for `S^{−3/2}` and one division, both standard.
  Because the regularised motion is smooth through a binary collision, Taylor's step does not collapse there, which is
  what regularisation buys any stepper.
- **Aarseth–Zare.** Its `Γ` keeps `−A B m_b m_c / |R₃|`, the unregularised third side, so it needs a square root and a
  division recurrence. Fine. AZ re-registers its reference body at every sync boundary, and a Taylor step never crosses
  a boundary (§3.4), so re-registration happens between steps, unchanged.
- **logH.** Its equations carry divisions by the kinetic or potential energy; recurrences exist. But logH regularises
  **through its leapfrog splitting**: a Taylor integration of the continuous time-transformed flow is a time-transformed
  Cartesian integration and may lose what makes logH regular. Taylor × logH needs its exact form written as TASK-M3-39
  writes the other logH pairs, and its behaviour at close approach is measured, not assumed.
- **`none` (Cartesian).** `r⁻³` by the power recurrence. The independent control stays independent. Near a collision
  Taylor's step shrinks with the distance to the singularity, as every unregularised stepper's does.

A triple collision stays singular for all of them; Taylor does not change that.

### 3.4 How it meets the existing step machinery

- **The sync grid is unchanged.** Steps are sized from `Δ_sync` (integrator contract, R-160) and never cross a sync
  boundary.
- **Landing on a boundary becomes exact.** Taylor's series is dense output: the state is known anywhere inside the
  step. The physical time `t(τ)` is one of the series, so the step that reaches a boundary solves `t(τ*) = t_sync` by
  Newton's method on the polynomial, with a fixed iteration cap (Part 2c rule 3), and evaluates the state there. This
  replaces the landing clamp and the secant iteration for this stepper; under the other steppers they stay as they are.
- **The predictive step limit is a measured question for Taylor, not inherited.** R-160's limit (AZ's form
  `dτ ≤ f·d_min/(|v_rel|_max·A·B)`; under Heggie, `dτ ≤ f·d_min/(|q̇|_max·dt/dτ)`) caps every step near an encounter
  at about 1/50 of the encounter time, whatever the order, and prin-rs measured it as the binding constraint ("`eta`
  is nearly inert above it"). Inherited as-is, it removes the long steps Taylor exists for. The wedges were RK4 failing
  to resolve pericentre; whether an error-controlled Taylor step needs the cap is unknown. **Taylor runs the wedge
  census (`config_stability`) with the limit on and off as two arms** before either becomes its default.
- **Events.** Collision and escape are sampled where they are now. Dense output allows locating `d_min` inside a step
  by the polynomial; that is optional and changes measured quantities, so it is its own decision (§6, open
  question 5).

### 3.5 Arithmetic: what emulation needs, and the guards

Double-float and quad-float rest on error-free transforms: TwoSum (`s = a + b` and its exact rounding error) and
TwoProd (`p = a·b` and its exact error). They are exact only under round-to-nearest, with add and multiply correctly
rounded, and with the compiler neither reassociating nor contracting them.

- **TwoProd uses Dekker's splitting, not a fused multiply-add.** The usual FMA form is wrong on this project's CI
  backend: R-547 exists because an explicit `fma` is **not fused on lavapipe** (RQ-298). Veltkamp/Dekker splitting
  needs only correctly rounded add and multiply. Every operation of every transform carries no contraction.
- **R-547's second clause applies: no subnormal intermediate in a branch input.** `df32` and `qf32` low limbs are
  routinely tiny. A low limb never feeds a branch input; a branch reads the high limb, or a value built under R-547.
- **Fast-math breaks the transforms silently.** Reassociating `(a + b) − a` to `b` turns the error term into zero,
  and the result looks right with half the digits gone. Metal's compiler defaults to fast-math.
- **The guarantees are per backend, and the route is SPIR-V → naga → MSL / SPIR-V / HLSL / WGSL** (canonical spec
  §1 item 2). Correctly rounded add, subtract and multiply, and 2.5 ulp division, are the **Vulkan** SPIR-V
  environment's; they do not bind Metal or the browser. Whether a `NoContraction` decoration survives naga's
  translation to MSL, HLSL or WGSL (WGSL has no equivalent) is unverified. Where `VK_KHR_shader_float_controls` exists,
  request `RoundingModeRTE` and `DenormPreserve`. For each backend, verify fast-math, contraction and rounding mode;
  none is assumed.
- **Division and square root** are a Newton step on a multiplication. The hardware reciprocal is fine as the seed;
  its result is never used unrefined.
- **Exponent range and underflow, per limb.** `df32` keeps f32's range: its low limb (~2⁻²⁴·x) underflows below about
  2e-31 with flush-to-zero. `qf32`'s fourth limb (~2⁻⁷²·x) underflows below about **5e-17**, which is exactly the size
  of the close-approach quantities regularisation exists for. The check is on the quantities that enter `Γ*`, not on
  `Q` alone: `R_i = |Q_i|²` is the separation itself (prin-rs's two-body collision test reaches `5.4e-27`), and
  products such as `R₁R₂R₃` set `dt/dτ`. Each tier states the per-limb threshold and is tested on `R_i` and those
  products.
- **The floors** are R-71 / REQ-INT-105 and the integrator contract's parameter table. prin-rs records that
  `TINY·TINY` underflows at both f32 and f64, and which guard does the work. Each new type states its floors there.

**The canary is necessary, not sufficient.** Reassociation and contraction are decided per shader when it is compiled,
so a start-up test in a separate shader does not certify the production shader. Two checks, both committed:
- **In the production pipeline**, a known TwoSum and TwoProd residual is computed by the production kernel itself and
  written to the payload. A wrong residual flags the run, never a silent fallback.
- **At start-up**, the stand-alone canary, with its negative control: a deliberately reassociated build must fail it.

A backend that fails either refuses the `df32` and `qf32` tiers with a message.

### 3.6 Where it is offered: Result first, everywhere by choice

- **Result (R-572)** gains the ladder. Its Precision field lists every tier the hardware and the canary allow. Its
  timed 64-pixel pilot already gives the cost estimate; it now runs at the chosen occupant, so it prices `df32` against
  native f64 on the user's own machine. The horizon warning uses the chosen precision's `eps`.
- **Live view.** Defaults stay as now (f32 on the GPU, the tier table's occupants). Because the human asked for full
  customisation, the precision and stepper selectors appear wherever the occupant selector does, and a choice that will
  not hold the frame rate carries a warning, never a block (R-561).
- **The payload stays f32 and f64.** A Result computed above f64 stores its fields rounded to f64; the stored fields
  (classes, shapes, times, drift) do not need more. Its provenance records the precision and stepper it was computed
  at. Storing raw extended state is a separate option (§6, open question 6).
- **The defaults do not move.** f64 Result, Heggie × RK4 everywhere. The ladder is opt-in.

### 3.7 What a combination can and cannot buy: the honesty mark

The digits are worth something only if the stepper's truncation error is below them. Taylor(`auto`) matches the order
to the precision per step; the global error still accumulates and chaos amplifies it, so that is no guarantee. RK4 or
a Yoshida stepper at `qf32` or `dd64` will usually be truncation-bound. The Result pilot measures it with **two arms,
one knob each**: the pilot pixels re-run with the precision raised one rung at the same step, and with the step
halved at the same precision. If only the precision arm moves the answer, the combination is precision-starved; if
only the step arm does, it is truncation-bound; if both, both. The Result says so in its sidecar and window: it is a
measurement, not an error.

### 3.8 Independence of the reference

R-33's reference stays independent: Brutus-style (Bulirsch–Stoer with arbitrary precision and convergence gating), not
the shared kernel. Boekholt & Portegies Zwart and Liao's clean numerical simulation each rest on **self-convergence of
one method** (Brutus raising precision and tolerance; CNS raising order and precision). A Taylor × `mp(bits)` Result
is a second, independent method at arbitrary precision; two methods agreeing is this project's own strengthening on
top of that, not something those papers did. It adds to the reference and does not replace it.

### 3.9 Deep zoom: the decoder switchover depends on the precision

The human's question, 26 Oct 2026:

> "One thing also is when linearised decoder kicks in needs to be relative to the precision mode? Idk if that has been
> considered"

It has not. Nothing in the corpus ties the switchover to precision, and working through it turns up a larger issue,
item 4 below.

**What is in force.** R-90: "Switch to the linearised decoder when the full decoder's adjacent samples give
bitwise-identical ICs, with ℓ_switch = 20 as an upper bound (whichever comes first)." Deep zoom §2: the CPU computes
`x₀` and `J_D` in f64 (`J_D` by central differences); the GPU computes `x = x₀ + J_D·δ` in f32; "Floor moves from ~23
to ~50+". The terminal stop is `AT_F32_FLOOR`.

**Four things change with the precision `P`** (`b(P)` its mantissa bits: 24, ~48, 53, ~96, ~106, or MPFR's):

1. **`ℓ_switch = 20` is an f32 number.** The full decoder at `P` holds until roughly depth `b(P)` (prin-rs measured f32
   collapsing at 16–23 and every f64 path at exactly rung 48). At `df32` a fixed bound of 20 hands off to the linear
   path some 25 levels early, so the bound becomes `ℓ_switch(P)`.
2. **For a multi-word type, "bitwise identical" is the wrong trigger.** A double-word number can hold an arbitrarily
   small low word: `(1, 2⁻²⁰⁰)` is a valid normalised `df32` or `dd64` value. So adjacent samples in `df32`, `qf32` or
   `dd64` stay bitwise distinct far past `b(P)`, and R-90's trigger may never fire; past `b(P)` the distinct values are
   rounding noise, not resolution. For multi-word tiers the trigger and the floor are **accuracy-based**: the
   adjacent-sample step against `eps(P)·|x|`. Bitwise collapse stays the trigger for fixed-width types (f32, f64, MPFR
   at a set width).
3. **The linearisation must be accurate to `P`, not to f32.** Its error is about `‖D″‖h²/2`. At f32 that is negligible
   from depth 20 on (`h² ≈ 1e-12` against `eps ≈ 1e-7`). At `df32`, `1e-12` is a hundred times `eps`, so switching at
   20 would *add* error. Where `‖D″‖` is O(1) relative to `|x|`, the safe rule is `h² ≪ eps(P)`, depth above `b(P)/2`,
   and switching at the full decoder's loss of resolution (depth ≈ `b(P)`) satisfies it. Nothing states that the
   composite map's curvature is O(1), and resolution can also be lost early for other reasons (saturation, or tiny
   `q₁, q₂` at shallow depth: deep zoom §2's corner case, RQ-553). So test 11 **measures** the linearisation error at
   the switch rather than assuming it.
4. **The floor that matters is the integration's precision, not the decoder's.** The ICs are absolute numbers of order
   1, and the integrator starts from them. If two pixels' ICs differ by less than `eps(P)` relative to O(1), they are
   the same IC to an integrator at precision `P`, whatever decoder produced them. prin-rs measured exactly this: "the
   literal formula `x₀ + J_D·δ` all in f32 collapses on exactly the same curve as plain f32", and the linearised split
   bought ~24 levels only by summing in f64 ("~24 levels over f32 and none over f64"). So deep zoom §2's "~50+" holds
   only if the sum and the integration run at f64. On the GPU at f32 the floor stays near 16–23 with either decoder.
   **That looks like a conflict in the corpus as it stands and should go to `REVIEW_QUEUE.md`** with this
   measurement, whether or not the ladder is adopted. (It is not RQ-554, which is a different issue.)

**And the decode's own inputs.** `x₀` must be at `P` or above: "`x₀` computed in f64, so its f32 cast is the best
possible representation" holds only for `P` below f64, so for `dd64`, `qf32` and `mp` the CPU precompute runs at `P` or
above. `J_D` needs less: `J·δ` is a correction of size about `2^−ℓ`, so `J_D`'s relative accuracy need is roughly
`eps(P)·2^ℓ`. The corpus says only "central differences in f64" (deep zoom §2; dd_decoder §2). prin-rs's secant
collapse (tracking `h` to depth 44, quantising at 48, `0.000e0` at 52, at f64) came from taking the difference step as
the cell width; a **depth-independent step** of about `eps^{1/3}` in chart units does not collapse. So the requirement
is a depth-independent finite-difference step with its accuracy stated. Forward-mode AD through the decode (the
Taylor machinery of §3.2) is an option, not the requirement: the composite includes an iterative invariant solve and a
branching canonicalisation (deep zoom), and forward mode gives derivatives at working precision, not exactly.

**What follows.** Precision is the deep-zoom axis as much as the horizon axis. Going deeper than depth `≈ b(P)` needs
integration at a higher `P`. Estimated floors (depth ≈ mantissa bits at O(1) chart coordinates; for multi-word tiers,
by the accuracy criterion of item 2, not bitwise collapse):

| Integration precision | Deep-zoom floor, roughly |
|---|---|
| f32 | ~16–23 (measured) |
| `df32` | ~46 |
| f64 | ~50 (measured 48) |
| `qf32` | ~90 |
| `dd64` | ~100 |
| `mp(bits)` | ~bits |

At the chart origin there is no such floor (the standing prin-rs result: "the deep-zoom floor is a property of where
you zoom").

**Proposed rules:**

- The switchover trigger is R-90's adaptive one, by bitwise collapse for fixed-width types and by the accuracy
  criterion for multi-word ones. `ℓ_switch` becomes a per-precision backstop, `ℓ_switch(P)`, never below `b(P)/2`, and
  is a frozen value of the sim key (Part 2c rule 1).
- The linearised path computes `x₀` at `P` or above, and `J_D` with a depth-independent finite-difference step (or AD,
  optionally) to the accuracy stated in the paragraph after item 4.
- `AT_F32_FLOOR` generalises to `AT_PRECISION_FLOOR(P)`. The corpus already keys the response to which decoder is
  active (deep zoom §2: a collapse of the full decoder is a switch, of the linearised one the true stop). The ladder
  adds a third response: where a higher rung exists, the user is offered it ("raise precision to go deeper"), never
  moved silently. (prin-rs reached the same correction as `Path::has_more_precise_path()`; that is evidence, not the
  corpus.) Whether a deep quad may run at a higher precision than its parent, so that only the deep part of a view
  pays, is open question 7.
- The linearised decoder's job changes from "buys depth" to "buys speed, and lets the expensive nonlinear decode run
  once per quad at high precision while the samples run cheaply". It buys depth only when its sum and the integration
  sit one rung above the decode.

### 3.10 Uniform mode gets the same deep-zoom machinery

The human, 26 Oct 2026:

> "does all the deep zoom stuff work in uniform mode? it should because you are still using quads just you have same
> res everywhere instead of adaptive refinement"

**As specced, not quite.** Two places tie the deep-zoom machinery to adaptivity rather than to depth:

- **Deep zoom's layer table** puts the linearised decoder in layer 2, "adaptive refinement": "CPU→GPU adds the
  linearised-decoder uniforms". Quad-local coordinates are in layer 0's decoder contract, but the linearised decoder
  arrives only with the adaptive scheduler.
- **The Result (R-572) is organised in rows, not quads**: "every row takes its cell width from the global grid"
  (results §3.2). A row's sample positions are global grid coordinates. At f64 on the CPU that holds to about depth 48.
  On the f32 GPU path it is the absolute f32 sum that R-395 says bands past the switchover.

What that costs depends on the tier, by §3.9 item 4. At f32 and f64 the integration floor equals the decoder floor, so
the cost is small: banding between `ℓ_switch` (20) and about 23 at f32, and nothing before about 48 at f64. **The change
pays at the tiers above f64**, where a row-based global grid would lose the depth the precision was bought for, and in
speed at every tier, since the linearised decoder is cheaper than the full one.

**Proposed, for those reasons: uniform is a complete tree at one level.** A uniform run is the quadtree with every leaf forced to the same
level `L` and no criterion. That is exactly how prin-rs's `Mode::Uniform` worked: refinement off, the veto or level
stopping everything. Then:

- every sample is quad-local (`c`, `h`, `δ`), with the quad's centre and half-width computed on the CPU at or above the
  run's precision;
- each quad decides the switchover for itself by §3.9's rule, so a deep uniform view uses the linearised decoder where
  the full one collapses, exactly as an adaptive one does;
- results §3.2's determinism argument carries over unchanged: a quad takes its cell width from the global level, never
  its own extent, so a resumed or re-ordered run is bitwise the uninterrupted one. "Rows done" becomes "quads done";
  the progress, cancel and resume design is otherwise unchanged.
- Deep zoom's layer table should say the linearised decoder arrives with **depth**, not with adaptivity: it belongs to
  any quad past the switchover, uniform or adaptive.

Test: a deep box rendered uniform and adaptive at the same leaf level gives bitwise-identical samples per quad (same
`c`, `h`, decode path and switchover depth). Its control is a shallow box, where both use the full decoder and must
also agree.

## 4. Build order

1. **The arithmetic types and the canary.** `df32` and `qf32` as `Real` types, CPU first, tested against f64 and
   `mp` bitwise where exact and to their stated precision otherwise. Then on the GPU with the canary, on the Mac.
2. **The existing occupants at `df32` on the GPU.** Heggie × RK4 at `df32`, compared against the CPU f64 path pixel by
   pixel. Benchmark against f64 on the CPU. This alone gives f64-class Results at GPU throughput, if the benchmark
   says so.
3. **Taylor on the CPU at f64.** Heggie first. Its coefficients checked against independently derived derivatives, then
   the whole stepper against Heggie × RK4.
4. **Taylor at `dd64` and `mp` on the CPU.** The horizon measurement (§5, test 7).
5. **Taylor on the GPU at `df32` and `qf32`, if a benchmark says it pays.** The memory is the risk. Heggie's planar
   state is about 13 series; with the intermediates of `Γ*`'s derivatives, perhaps 50–100 series per pixel (to count).
   At order 20 that is roughly 8 KB per copy at f32, 17 KB at `df32` and 34 KB at `qf32`, and each pixel carries
   `E + 1` copies. Against that, a thread has about 1 KB of registers at most (255 registers, low occupancy) and about
   128–256 B at good occupancy. It will spill. A lower order (8–12) at `df32` is the likely sweet spot, or a
   Yoshida-6 at `df32` may win outright. Measure, do not assume.
6. **`az`, `logH` and `none` with Taylor**, each pair's form written and checked as Part 2b's pairs are.

Each step is useful alone.

## 5. Tests (each with what makes it fail)

1. **The canary (§3.5).** Fails if any error-free transform is inexact on the device. Its negative control, a
   reassociated build, must fail it.
2. **Emulated arithmetic against `mp`.** `df32` and `qf32` add, multiply, divide and square root over a fixed corpus
   including near-cancellation and near-underflow inputs, each within its stated precision. Fails on a wrong
   renormalisation or a hardware divide.
3. **Taylor coefficients against derivatives.** For Heggie, the k-th coefficient against the k-th τ-derivative computed
   by `mp` finite differences at k ≤ 4 and by the recurrence at a second, independently written order (the
   "transcribe twice" rule). As with the FD tests on record, a sign error shared by both paths would pass, so the
   anchor chain `Γ* == (E_enlarged − h)·R₁R₂R₃` → Cartesian energy is checked on the series too.
4. **Taylor's order.** The figure-eight closure error against step size at fixed order n reads slope n, measured
   while truncation exceeds round-off: at f64 a high order reaches the floor at once, so use a low n, or `mp`. A
   landing-by-Newton bug shows as a slope below n, as the clamp's absence showed as order 1.06 against 2.08.
5. **Precision against Taylor order: convergence.** Raising precision and order together until the answer stops
   changing (the CNS and Brutus criterion) converges on a set of Burrau pixels. Fails if raising precision moves the
   answer while raising order does not, or the reverse, past the stated tolerance.
6. **Orbit-catalogue closure per tier.** The threebodyorbits operator confirmed (26 Oct 2026) that the per-orbit
   files store only the f64 part of their double-double refinement, so `ref_*` tests the tiers up to f64 only. Above
   f64, the operator's `atlas-params.json` carries `ic_hp` and `period_hp` (30 to 180 digits) on 1,260 rows traced to
   their source paper. **It is held outside the repo** until its data terms are confirmed
   (`docs/incoming/2026-10-26_orbit_catalogues/`, unmerged), so this test waits for that or runs locally and commits no
   data. Use only rows with more digits than the tier needs: the 30-digit ones cannot test `dd64` (~32 digits) or
   `qf32` near its precision. Closure after `period_hp` should fall with each rung on short-period stable orbits. Fails
   if a tier does not improve on the one below. Where no `ic_hp` exists, re-converge by Newton's method at the tier's
   precision from the f64 values.
7. **Horizon per tier.** Philosophy §7.1 predicts 23, 52 and 105; `df32` about 46. Measure where two neighbouring
   rungs disagree on Burrau. The prediction is an estimate; the test records the measurement and fails only if a
   higher tier's horizon is *shorter*.
8. **The Burrau neighbourhood.** First, the nominal Burrau pixel at f64 against R-33's reference to t ≈ 60: one
   trajectory is within reach of f64 with a regularised code, and the test establishes how far. Then, at a tier whose
   measured horizon passes t ≈ 60, a map of the neighbourhood whose nominal pixel reproduces the disruption against
   Szebehely & Peters 1967. The map, not the single trajectory, is what the ladder exists for.
9. **Equivalence at f64.** Taylor × Heggie at f64 against Heggie × RK4 at f64 on the existing near-field and
   `config_stability` fixtures: outcome labels agree on the tame regions, and differences sit where both methods are
   already known to diverge (chaotic regions over t = 13). Fails if tame pixels disagree.
10. **Agreement across backends.** A `df32` Result on Metal and on Vulkan: every branch input agrees (R-547, which
    corrects R-34), and continuous values agree to `df32` precision. Not bitwise: the determinism law expects
    continuous values to diverge across backends (canonical spec §2, R-84).
11. **Switchover per precision.** On a fixed deep-zoom box at O(1) chart coordinates, at each tier: record the depth at
    which the full decoder's samples collapse, the depth the switch fires, and the linearised decode's error against an
    `mp` decode. Fails if the switch fires where `h²` exceeds `eps(P)`, or if the linear error exceeds `eps(P)`. Its
    control is the origin box, where nothing collapses.
12. **Integration floor per tier.** On the same box, by depth, measure the adjacent-sample IC step as integrated (not
    as decoded) against `eps(P)·|x|`, and for fixed-width types also count distinct ICs. Fails if a tier resolves
    adjacent samples past about `b(P)` by that criterion, which would mean the measurement reads the decoder and not
    the integration. A distinct count alone would fail on correct multi-word code (§3.9 item 2). Its arm with teeth is
    f32 with the linearised decoder: it must collapse at 16–23, as prin-rs measured.

## 6. Rulings touched and open questions

**Rulings it would amend:**

- **Canonical spec §1 item 3**, "The GPU is f32-locked". Proposed reading: the GPU's native arithmetic is f32 (and f64
  where the hardware offers it); extended precision on the GPU is emulated, opt-in, and guarded by the canary.
- **R-265**, the kernel is f32 and f64 only and `Dd` is parked. Proposed: the kernel's `Real` gains `df32`, `qf32`,
  `dd64` and `mp(bits)`. R-265's condition for unparking has two clauses, "a precision question needs it and Brutus is
  too slow". The case made here is the Burrau neighbourhood mapped past t ≈ 52 (§2), where a per-pixel Brutus run is
  too slow; the single trajectory does not meet the second clause.
- **Integrator contract Part 2c** is held, not amended: the Taylor order is a sim-key integer and its step rule sizes
  values, not branches (§3.2).
- **R-547** governs the emulated arithmetic's branch inputs and forbids an FMA-based TwoProd on this project's
  backends (§3.5).
- **R-572**, "f32 or f64 only" in a Result. Proposed: the ladder, with f64 still the default.
- **Integrator contract Part 2 and Part 2b**: Taylor joins the stepper axis; precision becomes a third axis; R-561's
  warn-never-block extends to it.
- **R-90**, `ℓ_switch = 20` as a fixed bound: per precision (§3.9).
- **Deep zoom §2**, "Floor moves from ~23 to ~50+" with an f32 sum on the GPU: contradicted by the prin-rs measurement
  (§3.9 item 4). This one stands on its own and goes to `REVIEW_QUEUE.md` even if the ladder is not adopted.
- **R-572 and results §3.2**: a Result's uniform grid is a complete tree at one level, run in quads rather than rows
  (§3.10).
- **Deep zoom's layer table**: the linearised decoder belongs to depth, not to layer 2's adaptivity (§3.10).
- **R-33** is unchanged; §3.8 states why.

**Open questions for the human:**

1. Unpark extended precision now, or only after M3 (the current integrator milestone) closes?
2. Is the live view allowed the ladder by choice (§3.6), or is it Result-only?
3. Taylor's default order rule: Jorba & Zou's, or a fixed order per tier?
4. What does the profile's `force_evals` mean for Taylor: a measured equivalent, or a new field?
5. Event location by dense output (§3.4): adopt it for Taylor only, or keep events where every stepper samples them, so
   steppers stay comparable?
6. Should a Result above f64 be able to store raw extended state, or rounded fields only?
7. Per-quad precision in the tree: may a deep quad run at a higher rung than its parent, so that only the deep part of
   a view pays for it? It costs the scheduler a precision field per quad and makes neighbouring quads differ in
   arithmetic, which R-547's determinism rules would have to cover.

## 7. Sources to transcribe (not yet checked against the papers)

- Jorba, À. & Zou, M. (2005), *A software package for the numerical integration of ODEs by means of high-order Taylor
  methods*, Experimental Mathematics 14(1). The order and step rule.
- Abad, A., Barrio, R., Blesa, F. & Rodríguez, M. (2012), *TIDES*, ACM Transactions on Mathematical Software 39(1).
  Taylor with MPFR.
- Hida, Y., Li, X. S. & Bailey, D. H. (2001), *Algorithms for quad-double precision floating point arithmetic*,
  ARITH-15. Double-double and quad-double.
- Thall, A. (2006), *Extended-precision floating-point numbers for GPU computation*, SIGGRAPH research poster.
  Double-float on GPUs.
- Joldes, M., Muller, J.-M., Popescu, V. & Tucker, W. (2016), *CAMPARY: CUDA multiple precision arithmetic library
  and applications*, ICMS 2016.
- Joldes, M., Muller, J.-M. & Popescu, V. (2017), *Tight and rigorous error bounds for basic building blocks of
  double-word arithmetic*, ACM Transactions on Mathematical Software 44(2).
- Boekholt, T. & Portegies Zwart, S. (2015), *On the reliability of N-body simulations*, Computational Astrophysics and
  Cosmology 2, 2. Brutus and self-convergence.
- Wang, L., Nitadori, K. & Makino, J. (2020), arXiv:2001.01221, §4.1: the Pythagorean (Burrau) problem integrated to
  t = 63.
- Heggie, D. C. (1974), *A global regularisation of the gravitational N-body problem*, Celestial Mechanics 10, 217.
  Already in the corpus.
- Li, X. & Liao, S. (2017 onward), clean numerical simulation: Taylor at high order with MPFR, three-body families.
  Already in the replication suite.
- Szebehely, V. & Peters, C. F. (1967), *Complete solution of a general problem of three bodies*, AJ 72, 876. The
  Burrau disruption at t ≈ 60.

## 8. What the independent review changed

An independent reviewer checked the first draft against the corpus, the prin-rs record and the literature on 26 Oct
2026. It confirmed the Heggie polynomial argument, the precision figures, the horizons, the deep-zoom contradiction
and the §3.10 premises. It found twenty problems, all applied above. The ones that changed the design:

- **TwoProd by FMA is wrong on this project's CI backend** (R-547, lavapipe): Dekker splitting instead (§3.5).
- **A start-up canary does not certify the production shader:** an in-pipeline residual as well (§3.5).
- **Bitwise collapse never fires for double-word types:** an accuracy-based trigger and floor for them (§3.9, test 12).
- **`qf32`'s fourth limb underflows near 5e-17,** and the bound has to be checked on `R_i`, not `Q` (§3.5).
- **Taylor as first drafted broke Part 2c rules 1–3:** the order is a sim-key integer, the step rule sizes values only,
  and the landing loop is capped (§3.2, §3.4).
- **Inheriting the predictive step limit would cancel Taylor's long steps:** measured on and off instead (§3.4).
- **The Burrau argument was overstated and R-265 misquoted:** the case is the neighbourhood map, where Brutus is too slow
  (§2, §6, test 8).
- **`J_D`'s secant collapse came from the step choice, not central differences as such:** a depth-independent step, AD
  optional (§3.9).
- Smaller corrections: the cross-backend test asserts branch agreement and `df32`-precision values, not bitwise
  (test 10); the honesty check uses one arm per knob (§3.7); the convergence claim about Boekholt and Li & Liao, the
  CAMPARY citation, consumer f64 rates, the register budget, test 6's data source, and several attributions.
