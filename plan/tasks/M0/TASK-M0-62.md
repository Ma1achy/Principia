# TASK-M0-62 — The WSL image's hosted build kept in the Actions cache, keyed on the recipe, built only on a miss (R-1004)

- **Milestone:** M0
- **Closes:** REQ-TOOL-233
- **Depends on:** TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines, the guard test included

## Goal
The owner's answer to RQ-807 (R-1004): "(3) + (4), and (i). No credential on the PC." The hosted `wsl-image` job of
`.github/workflows/win-gpu.yml` keeps its build in the Actions cache under `wsl-image-<recipe key>`, the recipe key the
one the PC's cache on E: already uses (#509), saved from a `wsl-register` dispatch on `main`, restored on a hit, built
only on a miss, and uploaded as now. The cache on E: stays as built (3); `wsl-proofs` rebuilds as now, so proof 5 is
unchanged (i); nothing new reaches the owner's PC, and no job on it holds a credential.

## References
- `decisions.md` § "R-1004 — RQ-807, the WSL image: (3) and (4), and (i); the cache on E: stays as built, the hosted `wsl-image` job keeps its build in the Actions cache keyed on the recipe, restored on a hit and built only on a miss, proof 5 unchanged, and no credential on the PC *(closes RQ-807; amends R-285 and R-326)*"
- `decisions.md` § "R-285 — CI caches only the cargo registry and the fixture pool, with per-job keys"
- `decisions.md` § "R-326 — Actions caches are saved only on pushes to `main`; pull-request jobs restore only *(amends R-285, R-320)*"
- `decisions.md` § "R-967 — CI caches the Mesa (lavapipe) packages of `gpu-lavapipe` and the release jobs, keyed on the package list, so a run no longer fetches them from Ubuntu's archive *(amends R-285)*"
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"

## Deliverables
- `.github/workflows/win-gpu.yml`, `wsl-image`: a cache step keyed `wsl-image-<recipe key>`, restored in every run,
  saved only from a `wsl-register` dispatch on `main`; the build steps run only on a miss; the upload unchanged. The
  header's lines on the image ("The image can't come from another run …") rewritten for R-1004, the replaced text kept
  in the commit message.
- `wsl-proofs` unchanged: it builds the image for proof 5 (i).
- `xtask/tests/ci_self_hosted.rs` (or the test that guards the workflow's caches): the new entry is the one cache saved
  from outside a push to `main`, and only from that dispatch on `main`; no self-hosted job gains a permission, a token or
  a secret.

## Acceptance tests
- `cargo test -p xtask --test ci_self_hosted wsl_image_cache` — the `wsl-image` job restores `wsl-image-<recipe key>`,
  builds only when the restore missed, and saves only when the event is `workflow_dispatch` on `main` for
  `wsl-register`; `wsl-proofs` still builds; controls: a save on a pull request or a push to another branch fails, a
  key without the recipe key fails, and a self-hosted job given `actions: read` or a token still fails (REQ-TOOL-233).
- A `wsl-register` dispatch on `main` after merge, then a second: the first saves the entry (a miss), the second
  restores it and skips the build, its time against a build's reported (433–602 s in Phase A); both run ids in the PR or
  its follow-up comment (REQ-TOOL-233).

## Notes
- Entered 13 Nov 2026 by R-1004 (applied per R-369: task scoping, a CI task of its own).
- The image is about 1 GB against the repository's 10 GB Actions cache: a new entry evicts the least recently used, and
  an entry used only by register dispatches goes after 7 days unused. Whether it survives between dispatches is
  reported, a measurement, never tuned.
- Option (1), a token for the PC's register job, is refused by the owner's "No credential on the PC"; (2), a release
  download, is not taken.
