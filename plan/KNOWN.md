# Known gaps and open findings

Carried over from the old repository's review follow-ups and open threads. Fix one when you're in that code;
delete its line when it's done.

- **Escape from a binary fires late.** The shape point of a hierarchy keeps turning with the inner binary's
  phase, so the chord over the 0.4 window falls only as `1/d`, `d` the third body's distance. Measured on
  `fixtures/escape/hyperbolic_ejection.toml` (masses 1, 1, 1, a circular binary of separation 1; not the kernel's
  `M = 1` gauge, and `shape` takes the masses as given): the chord is `0.558/d`, below the provisional
  `tau = 1e-3` only at `d = 558`, so it fires at `t = 529`, not by 20. The coefficient is the fixture's: it is
  `2·√(μ_ρ/μ_λ)·(a/d)·2 sin(0.2·ω)` for a circular binary of separation `a` and angular speed `ω`, at most
  `4·√(μ_ρ/μ_λ)·a/d`. Systems that fly apart settle much sooner (the app's start view: every sample by `t = 32`).
  The golden frame (horizon 3) has no escape. In plain words: within the app's horizon of 50, a single body
  leaving a binary is in most cases still drawn as `bounded`, so the bounded region of a picture contains
  escapes that are not labelled yet; only starts that fly apart and escapes from tight binaries are caught.
  `tau` and the window are provisional. The re-validation (TASK-M3-34) measured, on 64 × 64 of `config stability`,
  `config basin` and `deep interior`: at `tau = 1e-3` precision 1.0000 and recall 3.4% (config) and 14.8% (deep
  interior) against check 2's ground truth; no false positive at any `tau` up to `1.353e-2`; the gap between escapers'
  and bound runs' `|Δn̂|` is 1.08× pooled, 1.9× and 2.4× by the claim's statistic, not 383×. Its criterion, written
  before the run, did not set a new `tau`, so the owner decides (the PR's "For the owner").
  Burrau (PR #29, TASK-M3-24) is an instance: the march and the RK45 reference both leave the lightest body unbound
  from `t ≈ 59.6` of Szebehely & Peters' units, but the chord's envelope is `≈ 0.85/d` (log–log slope `−0.97`
  against the escaper's distance `d`), so the escape label fires at `t = 157.6` at `tau = 1e-3`, in `[50, 100]` from
  `tau = 3e-3` (96.5) and by 65 only from about `5e-2`, which has false positives. Since escape-live-label the default
  rule is Fewbody, the label live: Burrau's first label is at 77.2 naming body 0 and its live label at the horizon is the
  same, so the two Burrau label tests run again. Under the live label the closure rule flickers: on a single escape
  from a binary its chord crosses `tau` with the binary's phase, so its live label switched 23,991 times on 1,820 of
  the re-score's trajectories and stays correct on 1,128 of 7,450 escape truths.
- **The escape rule fires on a bound system with a light, slow inner pair.** `E_rel` tests one body against the
  other two as if they were one mass, whether or not they are the bound pair. Measured: masses
  `(0.001, 0.001, 1)`, the light pair on a circle of separation 1, the heavy body at 400 on a circular orbit;
  every pair bound, total `E = −3.0e-6`. The far member of the pair is the farthest body (`d = 400.10` against
  400), its `E_rel = +1.2e-4`, and the window chord is `4.5e-5`, so `escape` fires at the first window mark
  naming that member. At `d = 300` and 150 it does not (`E_rel < 0`). For a light equal pair, each of mass `m`
  at separation `a`, on a circular orbit at `D` round a heavy body `M`, it fires for roughly
  `0.34 < m·D/(M·a) < 0.5` (the physics review's band: below it the far member's `E_rel < 0`, above it the heavy
  body is the farthest and is bound); the measured case is at 0.4, and 300 and 150 are 0.3 and 0.15. For the
  same reason, when a heavy single leaves a light pair the escape's `detail` can name a member of the pair.
- **A stepper's collision test steps over `r_coll` inside one macro-step.** `N_sub` is chosen from the separation
  at the macro-step's start (`crates/kernel/src/driver/wrapper.rs`, the `march` loop), so an encounter that begins
  and ends inside one macro-step is taken in a few substeps that jump across `r_coll`, and the sample is labelled
  `bounded` or `escape` instead of `collision`. Measured on 48 fuzzed ICs (`tests/terminal.rs`'s totality seed),
  KDK at `N_max = 64`: at the production key (`dt_macro = 1e-3`, `r_coll = 1e-3`, 10 time units) 9 collisions,
  against 18 at `dt = 2.5e-4` and 17 at `1e-4`, where the count has converged, so about half are missed. At
  `r_coll = 0.01` over 30 time units: 1 at `dt = 0.02` (`N_max` 3 and 64 alike), 20 at 0.005 (12 at `N_max = 3`)
  and 30 at 0.002 (29). The engine's field marches Heggie, whose collision test runs after every regularised step,
  so the shipped picture is less exposed; the steppers are the occupants affected.
- **The inverse encode's T2 range.** `docs/contracts/principia_inverse_encode_contract.md` Part 1 still
  requires E's representative in `[0, 2π]`, but under the offset link `β = z_β + π/2` (which keeps z = 0 at
  the equilateral) the inverse returns β below π/2. The contract needs restating when the decoder's mirror
  step is removed.
- **`validation::heggie::decide_fd` indexes its table's first row unchecked.** No caller passes an empty
  table; it should return an error rather than panic.
- **The radial-collision gate's failure path is tested only through its helpers.** No test runs a faulted
  passage through the whole gate. Add one before relying on the gate at a checkpoint.
- **Windows Vulkan:** four suites are reported rather than asserted on the PC runner, pending the NVIDIA
  driver investigation (see `win-gpu.yml`).
- **`xtask` has no `dx12` feature**, so the golden runner and the `cargo xtask` suites cannot select DX12; add it
  with the DX12 compute path (TASK-M0-85).
- **DX12: wgpu reports no VRAM size**, so the session header cannot be written and
  `qa_compute_fast_math_header_is_the_dispatch` fails there for that reason, not the fast-math refusal. Engine
  suites built for DX12 also need `validation/dx12`. Both belong with the DX12 compute path (TASK-M0-85).
- **The docs carry old ruling ids and *Was* blocks** (about 10,000 ids, 900 blocks across `docs/contracts`
  and `docs/design`). They are correct, just noisy. Clean a passage when you next edit it; don't run a
  mechanical sweep, because the reasons in them matter.
- **Test file names** (`qa_TASK-…`) are historical. Rename when touched.
- **`xtask` has no `dx12` feature**, so the golden runner and the `cargo xtask` suites cannot select DX12; add
  it with the DX12 compute path (TASK-M0-85).
- **What was run at import** (Linux, lavapipe): fmt, clippy with and without `controls`, every lint, `xtask
  controls --list`, and the ledger, validation, kernel and engine suites, plus the xtask and render tests the
  move touched. **Not run:** the GPU-kernel suites (they need `cargo xtask build-kernel`, which CI's
  `gpu-kernel` job runs on main), the GUI's and the remaining render and xtask tests (CI runs them on the first
  PR), and the WSL trust tests that need `ssh-keygen`.
- **`xtask` has no `dx12` feature**, so the golden runner and the `cargo xtask` suites cannot select DX12; add it with
  the DX12 compute path (TASK-M0-85).
- **The lock's gate test does not run every G9 pair through its tilt and slice** (TASK-M2-65, PR #59). It runs latent
  to latent, the mixed chart to the mixed chart and latent to the mass simplex at rest (the rest chart's end untilted);
  latent to the exponential shape sphere and the `(L_z, K)` chart to latent run only as locked morphs; a mixed or
  invariant chart to a rest chart and to Burrau do not run through the gate at all. Region D and the time-axis pair
  wait on TASK-M2-68 and TASK-M2-83 (ignored placeholders in `path_lock.rs`, whose controls are bare panics because
  the controls lint needs one per test). The gui's chart switch under a lock still takes `Segment::chart_switch`'s
  labelled jump; `Segment::locked_switch` needs the chart's leaves in the gui adapter.
- **The GPU path does not read the stop option** (`SimUniforms::stop_on_escape`): neither the dispatch nor the WGSL
  kernel carries it, so the CPU and GPU fields agree today only because `SimConfig` cannot set it. The GPU path must
  read it before `SimConfig` can set it, or the two fields will disagree.
- **The preset library is partly built** (TASK-M7-17). Built: the preset files (`render/presets/`), the loader, locked
  debug presets that fork on the first edit (`engine::contract::stain::preset`), the catalogue's outcome entry reading
  its file, the addressing presets and two bivariate presets. Not built: the tile-index and sample-index grey presets
  (`ctx.tile` carries no index; adding one moves every assembled text); the debug views generated from the ledger, which
  keep the catalogue's one wiring rule (`presets::named_view`) rather than one file each, and which §9.6's six groups
  file all as diagnostic; the other bivariate presets (n̂ × diffusion, n̂ × ensemble spread, outcome × FTLE apart from
  n̂'s); the `fuzziness` and `map_gradient` post nodes (a post sees one fragment, so the map gradient needs a pass over
  the colour buffer); the OKLab-uniform swatch set listing. The window needs: a preset list by group with a lock badge,
  `Loaded::apply` for every stain edit (so the first edit on a locked preset forks), and the preset's `measure` text.
