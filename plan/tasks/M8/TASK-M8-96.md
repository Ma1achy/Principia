# TASK-M8-96 — The v1 web skeleton measured on Pages (R-627): the worker layout, COOP/COEP, SharedArrayBuffer and wasm threads, text input, the clipboard and the frame rate, against RQ-550 and RQ-551

- **Milestone:** M8
- **Closes:** REQ-SYS-092, REQ-SYS-093, REQ-GUI-401, REQ-PERF-115
- **Depends on:** TASK-M8-86
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
On the v1 web skeleton (TASK-M8-86) as deployed to the GitHub Pages site, https://ma1achy.github.io/Principia/, by
`gh workflow run pages.yml --ref main` after TASK-M8-86 merges, measure, in each browser of R-110's pair (Chrome stable
and Safari) on the human's Mac, from the deployed Pages URL: where egui ran (a Web Worker on an OffscreenCanvas, or the
fallback that does run, egui's own web runner on the main thread) and, for a layout that did not run, what stopped it,
observed and reported, not argued; whether the COOP/COEP workaround, a self-hosted `coi-serviceworker` beside
`index.html`, makes the page cross-origin isolated, and whether SharedArrayBuffer and wasm threads then actually work;
text input and the clipboard; and the frame rate. The results are numbers and observed outcomes, recorded in
systems_architecture §3 under "Measured on the v1 web skeleton" and pointed at from RQ-550 and RQ-551, which stay open
for the human (split from TASK-M8-86 by code review 5663981956 of R-627's port, F1, applied per R-369).

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-571 — The browser: the native app first, then v1 web, the whole native app compiled to run in the browser on the GitHub Pages site, then v2 web, a separate product designed later *(replaces the browser item of the gap sweep's `INSTRUCTION_now.md` backlog; queues RQ-550 and RQ-551)*"
- `docs/contracts/principia_canonical_spec.md` § "0. What Principia is"
- `docs/contracts/principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker (and that worker is the wasm engine)"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-146 — The crate layout is confirmed *(closes RQ-76)*"

## Deliverables
- The measurements, taken on the deployed Pages URL with TASK-M8-86's measurement panel and scripted pan, as merged and
  deployed; this task changes no code under `web/` or `crates/`, so what is measured is what is deployed.
- The results, under "Measured on the v1 web skeleton" in `docs/design/principia_systems_architecture.md` §3, each with
  its browser, version, machine, layout and method, and the numbers in the PR.

## Acceptance tests
- Review checklist (gui reviewer), on the Pages URL after a manual deploy from `main` — the mock app opens in Chrome stable and Safari on the human's Mac, with its "mock engine" tag; the report gives, per browser and version, where egui ran (a worker on an OffscreenCanvas, or the fallback) and, for a layout that did not run, the error or missing API observed (REQ-SYS-092).
- Review checklist (code reviewer), on the Pages URL — with the service worker active and again unregistered, in each browser: `crossOriginIsolated`, `SharedArrayBuffer`, and the wasm-threads probe, each as observed; `coi-serviceworker.js` is served from the site's own origin; control: a result taken from a local server fails the review (REQ-SYS-093).
- Review checklist (gui reviewer) — per browser and layout: ASCII typing, a non-ASCII character, an IME composition, copy out of and paste into an egui text field, each works or does not, with the mechanism it took and any permission prompt; the IME and cross-application paste cells in Safari are the human's recorded result (see Notes) (REQ-GUI-401).
- Review checklist (perf reviewer) — per browser, version, layout and window size, the mean and 5th-percentile frames per second over at least 30 s of the scripted pan and of idle, beside `cargo run -p gui --features mock` on the same Mac (REQ-PERF-115).

## Notes
- R-627 (27 Oct 2026): a track task on the mock engine (R-390): on spare agent slots, never holding up the physics
  chain, merging once its reviews pass and CI is green without waiting for the gates of M3 to M6 (M2 and M7 by R-415's
  test). Reviewers code, qa, gui, and perf for the frame rate (applied per R-369).
- **Measure, don't argue.** Each result is a number or an observed outcome with its browser, version and method. The
  task settles neither RQ-550 nor RQ-551 and waits on neither; where a result bears on an option of either, the report
  says which, and the human rules. Text input and the clipboard are systems_architecture §3's rows 1 and 8; the worker
  layout is RQ-551's option (a).
- **It starts from the manual deploy** (R-627): the orchestrator runs `gh workflow run pages.yml --ref main` once
  TASK-M8-86 merges, and this task is dispatched on that deploy; the PR names the deploy's workflow run and the commit it
  built. A gap in the skeleton found while measuring (something the panel lacks, or a defect that stops a measurement)
  becomes a new task, split in the plan first, with its own PR and reviewers; this task then depends on it and is
  re-dispatched on the manual deploy after it merges (applied per R-369, code review 5664094727 of R-627's port, G2;
  `plan/WORKFLOW.md` § "The unit: one task, one branch, one PR", § "No deferral"). This task changes no code.
  *Was (before G2):* "If a measurement needs something the panel lacks, that is a finding against the skeleton, fixed on
  `main` and deployed again by hand before the measurement is taken, not a code change in this task (applied per
  R-369)."
- Browsers run on the human's Mac (R-186); a script drives every cell it can (Playwright on Chrome stable, safaridriver
  on Safari). The cells no script can drive, an IME composition and a paste from another application in Safari, are
  the human's: the human does them on the deployed site, batched like calibration values (R-71), at the M8 gate or
  sooner when the human chooses, and each such cell's acceptance is the human's recorded "works" or "does not", entered
  in systems_architecture §3 with browser and version. The PR lists those cells and the steps; every scriptable cell
  stays scripted, and the M8 gate does not pass until the human's cells are recorded (gui review 5664362471 of R-627's port, G4, applied per
  R-369). *Was:* "a script drives what it can (for example Playwright with WebGPU enabled), and the rest is by hand,
  each so marked."
- **What to try.** The PR description has a "What to try" section: the Pages URL, the measurement panel, and typing and
  pasting into a text field.
- **Setup outside the repo (R-349; gui review 5664362471 of R-627's port, G3).** The build and the browser runs need,
  on the human's Mac, each done or approved by the human, since they touch `~/.rustup`, `~/.cargo` or the system
  (`plan/HUMAN_SETUP.md` §6): the wasm target, `rustup target add wasm32-unknown-unknown`; `wasm-bindgen-cli` at the
  version of the `wasm-bindgen` crate in `Cargo.lock` (0.2.129 today), `cargo install wasm-bindgen-cli --version
  0.2.129 --locked`; Node.js for the browser runner, `node --version` 20 or later (the floor the pinned Playwright
  declares) (`brew install node` if absent), the runner itself, Playwright at a pinned version, installed in the repo
  by `npm ci` and driving the installed Chrome stable (`channel: "chrome"`, so no Playwright browser download); and
  Safari's WebDriver, `sudo safaridriver --enable`, then in Safari Settings → Advanced → "Show features for web
  developers" first, and only then Develop → "Allow Remote Automation". Until they are done, the task cannot run its
  local gui line or any browser measurement.
- *Was (gui review 5664362471 of R-627's port, G4), the REQ-GUI-401 acceptance line's end:* "each works or does not, with the mechanism it took and any permission prompt (REQ-GUI-401).".
