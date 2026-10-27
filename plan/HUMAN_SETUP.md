# Human setup: what a person does before the build starts

The build assumes these are in place. The agent can't do them, except where noted. Each cites the ruling it serves.

## Runner registration: not needed (R-186)

CI runs on GitHub-hosted runners: `ubuntu-24.04`, pinned by R-375 (CPU suites, lavapipe), and `macos-15` (Metal
correctness suites).
There is no self-hosted runner to register. Benchmarks and performance gates run on your own Mac via `prin profile`, at
milestone gates and on demand. If the first Metal check on `macos-15` fails or is flaky, the build stops with a
REVIEW_QUEUE entry proposing a self-hosted runner. The agent then scripts its setup for your approval.

## 1. Outside-contributor approval (R-174, R-186)

The repository is public. In **Settings → Actions → General → Fork pull request workflows from outside
collaborators**, choose **"Require approval for all outside collaborators"**. (R-174's same-repository `if:` guard is
dormant while there is no self-hosted runner, R-186; the approval setting still applies.)

## 2. Branch protection on `main` (R-175, R-177)

In **Settings → Branches → Add branch protection rule** (or a ruleset) for `main`:
- require a pull request before merging;
- require status checks to pass: `ci`, `pr-check`, `reviews-complete`, `xtask-ci`, `gpu-metal` and `gpu-lavapipe`
  (R-266; each appears in the list after its workflow has run once);
- require `mutants-check` too (R-305), the per-PR mutation gate's verdict over every shard (TASK-M0-23). It reports on
  every pull request, and passes when the diff has no mutant. The human authorised the agent to add it, once TASK-M0-23
  merges, and this is the one settings change R-305 allows;
- leave "require branches to be up to date before merging" off (R-266): CI runs on each PR merged with main, and every
  push to main runs CI again;
- turn on "do not allow bypassing the above settings", so the required checks bind every merge, the agent's included
  (R-266).
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
   web developers", and only then Develop → "Allow Remote Automation", which that setting reveals.
