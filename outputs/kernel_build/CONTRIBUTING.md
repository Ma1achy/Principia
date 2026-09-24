# Working agreement — prin-rs

How this repo is built and reviewed. Read alongside `BRIEF.md`, which is the specification;
this file is the process.

---

## 1. Repo layout

```
BRIEF.md              the specification. The entry point. Read in full before writing code.
CONTRIBUTING.md       this file.
CLAUDE.md             agent working notes (see §6).
reference/            the validated NumPy implementation + its own README. DO NOT EDIT.
src/, Cargo.toml      the Rust kernel.
```

**Move the six `tb_*.py` / `refine_test.py` modules into `reference/`** — they are currently at the
repo root and will be lost among `src/` and `Cargo.toml`. `reference/README.md` is already written as
that folder's readme.

**The root `README.md` should describe the project**, not the reference implementation. One
paragraph: what this renders, why uniform-resolution, and a pointer to `BRIEF.md`.

**`reference/` is read-only.** It is the ground truth the Rust is checked against. If something in it
looks wrong, raise it — do not fix it in place, because every number quoted in `BRIEF.md` was measured
with that exact code.

---

## 2. Scope — what "done" excludes

`BRIEF.md` §9 defines done. Restating the exclusions because they are the easiest thing to drift on:

**No quadtree. No scheduler. No GUI. No interaction. No streaming. No async.**

Uniform grid, one pass, write files, exit. Every one of those omissions is deliberate: this program
exists to make measurements the NumPy harness cannot, and adaptive machinery would make it harder to
trust, not easier to use. **If it grows a scheduler, that is a bug, not progress.**

---

## 3. Build order

Non-negotiable, because each step is only verifiable once the previous one holds.

1. **CPU, f64, single-threaded.** Match `reference/` to `~1e-10` on a small grid.
2. **Acceptance tests** (`BRIEF.md` §5) all passing.
3. **Rayon over pixels.** Confirm identical output to single-threaded.
4. **Generic over f32/f64 from one source.** Both exercised.
5. Only then: performance.

**Do not start step N+1 while step N is unverified.** A port that has not matched the reference is
not a port; it is a different program that compiles.

---

## 4. Pull requests

**One concern per PR.** "Port AZ" and "add PNG output" are separate.

**Every PR description must include the acceptance-test output as pasted text.** Not "tests pass" —
the actual numbers:

```
radial collision:   d_min = 1.35e-11   |dE/E| = 6.2e-15
gauge invariance:   alpha 0.25/1/4  ->  shape spread identical to 1e-10
error_ratio @t=13:  1.0000
burrau constants:   M=12  R=2.2361  E=-12.8167
reference match:    max |dr| = 3.1e-11  (8x8 grid, t=13, f64)
```

**The numbers are the review.** They can be checked against the reference far more reliably than a
transcription of the equations of motion can be eyeballed.

**Say what you were unsure about.** A PR that flags "I wasn't certain the reference-body tie-break
matches" gets a useful answer. One that doesn't, gets a slower review.

---

## 5. Review protocol

Reviewed against the **physics and the reference implementation**, not for compilation or idiom — the
reviewer cannot run the Rust. Most useful on: is the AZ algebra transcribed correctly, does the
outcome encoding match the spec, has an invariant been quietly broken. **For Rust style, trust the
compiler and clippy over the review.**

Checked in this order, and review stops at the first failure:

1. **Does it match `reference/` at f64?** Everything else is unverifiable until this holds.
2. **Is the Hamiltonian finite-differenced against the analytic derivatives?** This is the test that
   catches sign errors, and **sign errors in AZ are invisible** — the symptom is drift that does *not*
   fall when you shrink the step. If this test is absent it will be flagged before anything else is
   read.
3. **Acceptance tests** (§5 of the brief) — radial collision, gauge invariance, Burrau constants.
4. **Are `r_coll` and `epsilon` canonical and fixed at `t=0`?** And is the triple rule a **count**
   (`>= 2` pairs), not "all three"?
5. **Does `error_ratio` use MAD internally and `max` aggregation?** Does anything discard a copy?
6. **Scope creep.**

---

## 6. Invariants — breaking any of these is a correctness bug

These were each established by measurement and several were learned by getting them wrong. They are
not preferences.

1. **Uniformity: every pixel carries exactly `E+1` copies at all times. Never discard one.** A badly
   integrated trajectory is a *measurement outcome* ("this could not be determined"), not missing
   data. Discarding biases the sample toward tame trajectories, which on a chaos instrument is exactly
   backwards. It also turns the `event` normalisation constant `1 - 1/(E+1)` into a per-pixel
   variable.
2. **Scale invariance: every length in canonical units, fixed at `t=0`, never co-moving.** An absolute
   length breaks the dynamical similarity this project quotients out — measured 1.66× error from an
   arbitrary choice of overall size. A *co-moving* length is worse: it makes the Hamiltonian
   time-dependent and destroys energy conservation.
3. **Determinism: jitter seeded from `(i, j, seed)`, never a global RNG.** Any pixel must be
   reproducible in isolation.
4. **One grain for events: `class ⊕ detail` everywhere.** Do not add a class-only variant — argmax does
   not commute with masking, so the two grains disagree in ways that are hard to see.
5. **`error_ratio` uses MAD internally and `max` across pixels.** A standard deviation returns NaN on
   precisely the pathological pixel it exists to flag.
6. **No `L_z` error meter.** From rest `v = 0`, so `L_z = 0` for every copy and `sigma_Lz(0) = 0` — the
   ratio is `0/0`, structurally undefined for this whole configuration family.

---

## 7. Failure signatures worth memorising

Three bugs in this project failed **silently and looked like physics**. Each has a tell:

| symptom | means |
|---|---|
| **drift that does not fall when you shrink the step** | a **wrong equation**, never a step-size problem |
| a region "intractable", enormous runtime, tiny steps | a step-size rule **duplicating** what the method already does — AZ's time transformation already shrinks the physical step at close approach |
| runtime ~100× nominal on some pixels | non-finite values never satisfying a loop exit (`NaN >= x` is `false`) — test `is_finite` explicitly |

**If a result looks physically impossible, check the machinery before believing it.** The energy
control exists for exactly this: `error_ratio` has a known correct value of exactly 1.0, so any
departure is a bug in the code, not a discovery about the system.

---

## 8. Open questions — judgement wanted, not just implementation

**Flag these in PRs rather than silently choosing.**

1. **Shared vs per-copy AZ reference body.** AZ picks a reference per trajectory and it can change
   mid-run. Should all `E+1` copies of a pixel share the nominal copy's reference? Unshared references
   may give copies differently-*structured* error, corrupting the ensemble spread while leaving each
   trajectory's own drift healthy. **Implement as a flag; measure both.** This is a live dispute — one
   measurement says f32 AZ's raw drift is fine, another says the ensemble diagnostic breaks early, and
   this is the leading hypothesis for the discrepancy.
2. **RK4 is not symplectic or time-symmetric.** It was chosen to prove the physics, not to ship. If a
   symplectic or time-symmetric alternative is straightforward, prefer it — **but match the reference
   at f64 first, then change one thing at a time.**
3. **f32 viability of AZ.** Unresolved and a major reason this program exists. Do not assume either
   answer.

**Disagreement is wanted.** Several conclusions in this project were overturned by whoever was closest
to the code — a normalisation defect, an estimator degeneracy, a meter that was structurally undefined.
**If a framing in `BRIEF.md` is wrong, say so in the PR.** That has consistently been worth more than
the code.
