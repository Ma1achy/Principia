# Human setup: what a person does before the build starts

The build assumes these are in place. The agent can't do them, except where noted. Each cites the ruling it serves.

## Runner registration: not needed (R-186)

CI runs on GitHub-hosted runners: `ubuntu-24.04`, pinned by R-375 (CPU suites, lavapipe), and `macos-15` (Metal
correctness suites).
One self-hosted runner is approved, `malachys-pc-rtx2080s` on your PC, for measurements only (R-711); no other is registered. From the anchor move it may run what R-859 lists, as named measurements and never as required checks; the at-the-PC items don't block that. Benchmarks and performance gates run on your own Mac via `prin profile`, at
milestone gates and on demand; Windows benchmark numbers are a separate named series, never mixed with the Mac's (R-859). If the first Metal check on `macos-15` fails or is flaky, the build stops with a
REVIEW_QUEUE entry proposing a self-hosted runner. The agent then scripts its setup for your approval.

## 1. Outside-contributor approval (R-174, R-186)

The repository is public. In **Settings → Actions → General → Fork pull request workflows from outside
collaborators**, choose **"Require approval for all outside collaborators"**. (R-711 bars every pull-request trigger on its
one self-hosted runner, in place of R-174's same-repository `if:` guard; the approval setting still applies.)

## 2. Branch protection on `main` (R-175, R-177)

In **Settings → Branches → Add branch protection rule** (or a ruleset) for `main`:
- require a pull request before merging;
- require status checks to pass: `fmt`, `clippy`, `build`, `ci`, `gpu-lavapipe`, `gpu-metal`, `pr-check` and
  `reviews-complete` (R-908 and R-909, the owner's CI model approved on 9 Nov 2026; his order of 8 Nov 2026: "PR
  required checks become: build, fmt, Clippy, the product test suite, a lavapipe GPU smoke set, reviews"; `gpu-metal`,
  the Metal smoke set on a GPU-path change, keeps its name; each appears in the list after its workflow has run once).
  `ci.yml` bears the first six, each a job of every pull-request run that skips its work inside the job on the
  classification's verdict (R-656), so each reports on every pull request. The owner sets branch protection himself,
  from the orchestrator's old-to-new list (R-909); `xtask/tests/ci_changes.rs` checks that this list is the one every
  required check is tested against.
  *Was, until the owner switches it (R-907 to R-909):* `ci`, `pr-check`, `reviews-complete`, `xtask-ci`, `gpu-kernel`,
  `gpu-metal` and `gpu-lavapipe` (R-266, and `gpu-kernel` again since TASK-M0-14), and `mutants-check` (R-305), the
  per-PR mutation gate's verdict, which the owner's CI model takes off the pull request's path with `xtask-ci` and the
  full tier's GPU suites (they run on each merge to `main`; R-907, R-908). The human authorised the agent to add
  `mutants-check` once TASK-M0-23 merged, the one settings change R-305 allows;
- leave "require branches to be up to date before merging" off (R-266): CI runs on each PR merged with main, and every
  push to main runs CI again;
- turn on "do not allow bypassing the above settings", so the required checks bind every merge, the agent's included
  (R-266).
  *3 Nov 2026 (R-682):* the owner allows admin bypass on `main`, so that he can merge by hand when a required check
  hangs. Agents never use it, and the required checks still bind every merge an agent makes.
- turn on "require conversation resolution before merging" (R-276): a PR merges only when every review thread on it
  is resolved.

Leave "require approvals" off: the reviewers are agents posting `VERDICT:` reviews from one account, which GitHub
doesn't count as approvals, and `reviews-complete` is the check that does (R-175).

## 3. PR labels (R-180, R-177): done by the agent

`design`, `investigation`, `validation` (pr-check, TASK-M0-03) and `gui`. They were created with `gh label create`
when the R-168 to R-184 rulings were applied; nothing to do unless they're missing.

## 4. Confirm the crate map (R-170): done

Confirmed by R-185 (26 Sep 2026); TASK-M0-00 is done.

## 5. GitHub Pages (R-627): done

The human's decision of 27 Oct 2026 (R-627): GitHub Pages is enabled with source "GitHub Actions" (build type
"workflow"), at https://ma1achy.github.io/Principia/. The `github-pages` environment's deployment branch policy is
custom, with one branch policy, `main`. The orchestrator applied it on the human's word, and no other repository
setting was changed. The site is deployed only by a manual (`workflow_dispatch`) run from `main` through that
environment; there is no automatic deploy on merge until the human says v1 is ready to be found.

## 6. The v1 web toolchain on the Mac (R-627, R-349): to do before TASK-M8-86 runs its local check

TASK-M8-86 (the v1 web skeleton) and TASK-M8-96 (its measurements) need these on the human's Mac. Each touches
`~/.rustup`, `~/.cargo` or the system, so the human does it or approves it (R-349); no agent runs them unasked. Until
they are done, TASK-M8-86 cannot run its local gui check and TASK-M8-96 cannot measure (gui review 5664362471 of
R-627's port, G3, applied per R-369).

1. The wasm target: `rustup target add wasm32-unknown-unknown`.
2. `wasm-bindgen-cli` at the version of the `wasm-bindgen` crate in `Cargo.lock` (0.2.129 today; TASK-M8-86's PR names
   the version it pins): `cargo install wasm-bindgen-cli --version 0.2.129 --locked`.
3. Node.js 20 or later (the floor the pinned Playwright declares) for the browser runner: `node --version`, and `brew
   install node` if it is absent. Playwright itself is pinned in the repo and installed there by `npm ci`; it drives
   the installed Chrome stable (`channel: "chrome"`), so no Playwright browser download is needed.
4. Safari's WebDriver: `sudo safaridriver --enable`; then in Safari, first Settings → Advanced → "Show features for
   web developers", and only then Develop → "Allow Remote Automation", which that setting reveals. Before each Safari
   run (the WebDriver check, in both layouts), **quit Safari first**: the automation session takes over Safari, and a
   window the human is using, such as a research session, collides with it. The human quits it; an agent never does
   (R-674, R-349).
