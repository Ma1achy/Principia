# Cold-read review of the build plan

*25 Sep 2026 · a review, not a change: nothing else in the repo is edited by it.*

**Scope.** Read in order: `docs/read_first/principia_INDEX.md`, `plan/WORKFLOW.md`, `plan/MILESTONES.md`, `decisions.md`
(skimmed). Dry-run, from each task file and only the docs it cites: TASK-M0-01 to TASK-M0-05, TASK-M2-08 (the shape-sphere
chart), TASK-M3-12 (the escape detector), TASK-M7-02 (colour spaces). Live repository state was read with `gh` (read-only).
`python3 plan/check_plan.py` is green (244 tasks, 1220 live requirements, 0 failures), so nothing below is something the
checker already catches.

**Grades.** **blocker**: the task can't be completed or verified as written. **should-fix**: two readings are possible, or a
PR is likely to be wasted. **note**: small or stale.

Citations are `file:line` at `main` (`a59a772`).

---

## 1. Gaps: what a task needs that its references don't give

- **G1 · blocker: no task adds the GPU CI jobs.** `plan/tasks/M0/TASK-M0-04.md:30` requires `gpu_harness` "in CI on the
  self-hosted Metal runner and on lavapipe". M0-04's deliverables (`:22–27`) touch no workflow file, and
  `plan/tasks/M0/TASK-M0-01.md:25` specifies a single-job `ci.yml` (build, test, deps, `xtask ci`). No task installs Mesa
  lavapipe, targets a `self-hosted, macOS, ARM64` runner, or says how `GpuHarness` is told which backend to use
  (`TASK-M0-04.md:37` says only "selects either by backend").
- **G2 · blocker: the M0-01 dependency table has no node→crate map.** `TASK-M0-01.md:12,24` asks for an allowed-edge table
  "against the build DAG of systems_architecture §7". But §7 is titled "abstract"
  (`docs/design/principia_systems_architecture.md:251`) and says it is "the *can't-exist-before* graph, not the milestone
  plan" (`:253`). Its nodes (link registry, decoder, canonicalise, chart system, validation, resolve/lowering, dispatch,
  payload, compositor, …) aren't assigned to crates anywhere. So the implementer would be inventing the edges that
  REQ-SYS-004 is checked against.
- **G3 · should-fix: the controls gate trips on tests that came before it.** `TASK-M0-04.md:26,32` makes
  `cargo xtask controls` fail when "a test has no control". `TASK-M0-01.md:31`, `TASK-M0-02.md:31` and
  `TASK-M0-03.md:28–31` add xtask tests but register no controls. M0-02 and M0-03 depend only on M0-01, so they can merge
  before or after M0-04, and M0-04 doesn't list retrofitting them. Three things are also unspecified: how xtask tests
  reach the `negative_control!` macro in `crates/validation`, how a test is matched to its control, and what
  `cargo test --workspace --features controls` does in crates that don't declare the feature.
- **G4 · should-fix: `cargo xtask ci` has no notion of cadence.** `TASK-M0-01.md:12` makes `ci` the single entry point
  for "plan-check, controls, gate, golden, codegen, bench". But R-110 (`decisions.md:801–802`) and `plan/WORKFLOW.md:18–22`
  run benchmarks nightly and at gates, and screenshots on GUI PRs and at gates. `TASK-M0-19.md:23` still registers bench
  in `ci`, and `TASK-M0-06.md:35` registers screenshot "for GUI PRs and the milestone gates". Three pieces are missing: a
  scheduled workflow, a definition of a "GUI PR" (label or paths), and a trigger for a milestone-gate run and its report
  (`WORKFLOW.md:104–105`).
- **G5 · should-fix: M3-12 doesn't cite the rule its Goal invokes.** `plan/tasks/M3/TASK-M3-12.md:12` asks for
  "rule 6's explicit-fma treatment (R-34)". Rule 6 is `docs/notes/principia_gpu_determinism_note.md:52`, and the
  enumeration is `docs/contracts/principia_integrator_contract.md:356` (Part 4). The References (`TASK-M3-12.md:15–31`)
  cite neither. `principia_INDEX.md:42` speaks of Part 2c's "five rules", which makes "rule 6" harder to find.
- **G6 · should-fix: M3-12 gives no provisional `tau`.** Every acceptance test (`TASK-M3-12.md:40–46`) fires on
  `|Δn̂| < tau`, but the task file never gives a value. The provisional range 7.04e-05 … 2.70e-02 appears only in
  REQ-EVT-024's verify detail (`plan/requirements.yaml`). prin-rs ran `CLOSURE_TAU = 1e-3`
  (`docs/reference/prin-rs/src/outcome.rs:263`), and the task doesn't mention it.
- **G7 · note: M2-08 fixture tooling.** `plan/tasks/M2/TASK-M2-08.md:37` generates a fixture by running
  `ic_inspector.html`'s JS "under node". It doesn't say how the script is extracted from the HTML, which Node version to
  use, or whether CI regenerates the fixture or only reads the checked-in copy.
- **G8 · note: M7-02's reference is left to the implementer.** `plan/tasks/M7/TASK-M7-02.md:37` leaves the choice of
  Ottosson reference (URL, revision) to the implementer, and fetching it needs network access. R-51
  (`decisions.md:406–409`) names no source.

## 2. Ambiguities: two readings, or a reference that points wrong

- **A1 · blocker: the convergence gate's canonical failure passes under the obvious reading.** `TASK-M0-05.md:26,30`
  (and REQ-VAL-004) take 0.0947 → 0.2153 → 0.4423 → 0.5494 "at strides 0, 32, 4, 1" as the sequence that must fail
  "relative steps shrink monotonically". Read in the listed order, with relative step = |Δx| / |previous x|, the steps
  are **1.273, 1.054, 0.242**. They shrink monotonically, so the sequence fails only if the threshold happens to be below
  0.242. The failing shape that philosophy §4.5a (`docs/read_first/principia_00_philosophy.md:210–211`) and pitfalls §3
  (`docs/read_first/principia_01_pitfalls.md:228–230`) describe ("largest relative step at the finest stride") only
  appears if stride 0 is the finest, i.e. the order is 32, 4, 1, 0 (relative steps 1.054, 0.242, 0.828). Four things are
  unspecified: what "stride 0" means, which way the sequence refines, how a relative step is defined, and which step must
  fall below the threshold.
- **A2 · blocker: the placeholder threshold has no value.** `TASK-M0-05.md:27,30` needs "a fixture whose relative steps
  shrink monotonically below the threshold passes", but the placeholder is never given a number. REQ-VAL-135 isn't
  calibrated until M3, and the runner refuses a threshold that names no requirement (`:23`). So the implementer can't
  choose one without an R-71 proposal.
- **A3 · should-fix: M2-08's round trip meets the hemisphere fold.** `TASK-M2-08.md:42–43` expects (s, t) = (·, 0) → L⁻,
  and `shape_vec(decode(u,v)) == n(u,v)`. But `docs/design/principia_chart_reference.md:347–350` makes the chart a 2-to-1
  cover whose canonical decode keeps w ≥ 0, and M2-08 depends on M2-07, the canonicaliser. If "decode" includes C, the
  lower hemisphere fails the gate. The task doesn't say whether the gate composes Φ alone or Φ then C.
- **A4 · should-fix: "no M5 task flows sim data GPU→CPU" can't be judged.** `TASK-M0-02.md:38` (REQ-SCHED-001;
  `docs/design/principia_deep_zoom.md:70`) sits beside M5 building `QuadReduction` (`plan/MILESTONES.md:210`), which R-142
  makes "the sole automatic return" (`decisions.md:1026–1027`). Nothing says whether a reduction counts as "sim data", so
  the checklist line has no pass/fail rule.
- **A5 · should-fix: is pr-check section-level or item-level?** `TASK-M0-03.md:24` checks that mandatory *sections* are
  present. But `:29–30` fail a body in which one *individual* error meter or discriminator lacks its statement, which
  needs a per-item format that the template (`:22`) doesn't define.
- **A6 · note: citation of a whole file.** `TASK-M3-12.md:29` and `TASK-M7-02.md:18` cite `open-questions.md`
  § "Open questions", the file's title heading, so neither points at an entry.
- **A7 · note: screenshot runner argument.** It is `<artboard>` in `WORKFLOW.md:84` and `<suite>` in `TASK-M0-06.md:35`.
- **A8 · note: "naming the fault".** `TASK-M0-02.md:31` asks that an archive citation fail "naming the fault". Today
  `plan/check_plan.py:110` reports only "doesn't exist", because archived files are outside the citable index
  (`plan/tools/sections.py` `CORPUS_GLOBS`). M0-02's deliverables don't include changing `check_plan.py`.

## 3. Contradictions between a task, its docs and decisions.md

- **C1 · blocker: a `contract` crate that R-146 didn't confirm.** `TASK-M0-01.md:12,22` creates a `contract` crate, and
  `TASK-M0-08.md:28` lints `crates/{kernel,ledger,contract,engine}`. R-146 (`decisions.md:1047–1050`) confirms the layout
  proposed in RQ-76 (`REVIEW_QUEUE.md:1131–1135`), which has no such crate and puts the typed surfaces in the engine
  crate. `WORKFLOW.md:58–63`, `MILESTONES.md:63`, `docs/contracts/principia_gui_state_contract.md:15` ("defined **once**,
  in Rust, in the engine crate") and `TASK-M0-16.md:11,23` (`crates/engine/src/contract/`) all agree. The first task of
  the build contradicts the ruling on its own layout.
- **C2 · blocker: M3-12 closes a calibration whose evidence comes later.** `TASK-M3-12.md:47` closes REQ-EVT-024, whose
  proposal must place tau "inside the re-measured gap" (REQ-VAL-051). REQ-VAL-051 is closed by TASK-M3-34, and M3-34
  depends on M3-12 through M3-33 (checked in `plan/tasks.yaml`). The PR can't contain its own evidence.
- **C3 · should-fix: M3-12 contradicts itself on tau.** `TASK-M3-12.md:50` says "no calibration requirement covers"
  tau, while `:47` and `:52` close REQ-EVT-024, which is that calibration. The note is stale.
- **C4 · should-fix: stale notes on the crate layout.** `TASK-M0-01.md:35` calls the layout "pending the reviewer", and
  `MILESTONES.md:9–11` says the other crate boundaries are "for the build to set". R-146 settled both.
- **C5 · note: M2-08 cites both conventions.** `TASK-M2-08.md:21` cites R-12, whose text says "θ on `v`"
  (`decisions.md:115`), alongside R-14, which puts θ on the horizontal axis (`decisions.md:141–142`). chart_reference §3.3
  reconciles the two; the task doesn't say that R-14 wins.
- **C6 · note: plan-check, directly or through `ci`?** `WORKFLOW.md:15` has CI running `cargo xtask plan-check`
  directly, while `TASK-M0-01.md:12` routes every runner through `cargo xtask ci`.
- **C7 · note: stale file count.** `principia_INDEX.md:3` says "110 files in `docs/`"; the tree holds 173.

## 4. Acceptance tests that aren't concrete

- **T1 · should-fix: "~1e−14" is not a threshold.** `TASK-M2-08.md:43` (REQ-CHART-020; `principia_chart_reference.md:318,543`)
  needs a stated bound, or a calibration requirement.
- **T2 · should-fix: the convergence gate's fixtures.** The passing fixture has neither a threshold (A2) nor data
  (`TASK-M0-05.md:30`), and whether the failing fixture fails depends on the reading (A1).
- **T3 · should-fix: the M0-02 review checklists.** `TASK-M0-02.md:38–40` apply the checklists to MILESTONES and
  tasks.yaml, but A4 leaves the M5 GPU→CPU line without a criterion.
- **T4 · note: M3-12's escape fixtures.** `TASK-M3-12.md:37` names seven synthetic fixtures (hyperbolic ejection, grazing,
  flicker, …) with no construction or parameters. REQ-VAL-051's own detail warns that prin-rs found a gap of at best 6.8×
  rather than 383×.
- **T5 · note: a tolerance set by the same PR.** `TASK-M7-02.md:31` tests against REQ-COL-049, the tolerance that the same
  PR proposes. It doesn't say what value CI uses before the M7 gate confirms it.

## 5. Size and dependencies

- **S1 · should-fix: M0-06 is over the budget.** It is ~550 lines (`TASK-M0-06.md:9`), above the ~500 in
  `WORKFLOW.md:13,29`, and it bundles two runners: golden with its repro mode, and screenshot. R-156 allowed the overrun,
  but the task would still split cleanly.
- **S2 · should-fix: M0-02 and M0-03 should depend on M0-04 (see G3).** Otherwise the order of merges decides whether
  `controls` goes red.
- **S3 · should-fix: the M3-12 ↔ M3-34 inversion (see C2).** Either REQ-EVT-024 moves to M3-34, or it splits into a
  provisional tau (M3-12) and a confirmed tau (M3-34).
- **S4 · note: M0-04 carries three deliverables.** It holds a GPU harness, a shared proptest config, and a workspace-wide
  control registry with its runner, in ~400 lines. That is tight once G1 and G3 are added.

## 6. Setup a human must do first

- **H1 · blocker: a self-hosted runner on a public repository.** No runner is registered (`gh api
  repos/Ma1achy/Principia/actions/runners` → 0), yet R-110 (`decisions.md:803`) and M0-04 need an Apple-silicon Metal
  runner on every commit. The repository is **public**, and a self-hosted runner on a public repository runs code from
  fork PRs. The plan needs a policy: which events the GPU job runs on, and whether outside contributors need approval.
- **H2 · should-fix: no branch protection or required checks.** `main` has neither (`…/branches/main/protection` → 404),
  though `WORKFLOW.md:22` ("a red CI blocks review") and `:99` ("merge when every named reviewer has approved and CI is
  green") assume both.
- **H3 · should-fix: who the reviewers are.** `WORKFLOW.md:44–51,91–98` name the `code`, `qa`, `physics`, `gui` and
  `perf` reviewers and require each to "approve explicitly". It doesn't say whether they are people, agents or GitHub
  accounts. GitHub won't let a PR's author approve their own PR, so agent reviews run from one account can't produce the
  approvals.
- **H4 · should-fix: the PR labels don't exist.** `design`, `investigation` and `validation` (`TASK-M0-03.md:23`) aren't
  in the repository, which has only the GitHub defaults. pr-check also needs the `pull_request` event types `edited`,
  `labeled` and `unlabeled` so that it re-runs when a reviewer relabels (`TASK-M0-03.md:34`).
- **H5 · note: toolchains with no install step.** The plan assumes Python + PyYAML (M0-02 installs them), Node (M2-08),
  Mesa lavapipe (G1) and a pinned rust-gpu nightly (M0-14, not dry-run here). Only the first has a stated install step.

---

## Tally

| grade | count | ids |
|---|---|---|
| blocker | 7 | G1, G2, A1, A2, C1, C2, H1 |
| should-fix | 18 | G3–G6, A3–A5, C3, C4, T1–T3, S1–S3, H2–H4 |
| note | 12 | G7, G8, A6–A8, C5–C7, T4, T5, S4, H5 |

Rulings on these findings, if any, go in `decisions.md`. This file is left as the record of the read.
