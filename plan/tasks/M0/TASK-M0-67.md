# TASK-M0-67 — The WSL proofs print no tailnet address: one masking function in wsl.ps1 for every Report line, echo and failure

- **Milestone:** M0
- **Closes:** REQ-TOOL-240
- **Depends on:** TASK-M0-63
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~450 lines, the test file and its controls included

## Goal
The PC's side reported that proof 3's line printed the host's tailnet address, unmasked, in a public Actions log.
`.github/scripts/wsl.ps1` gets one masking function, `Hide-Tailnet`, through which everything it prints that can hold a
host address goes: each proof's line (and so the summary at the end and the `wsl-proofs.txt` artifact), its echo of
what `wsl.exe` prints, and its failure messages. An IPv4 address in Tailscale's range 100.64.0.0/10 prints as
`100.x.x.x`, an IPv6 address in its prefix fd7a:115c:a1e0::/48 as `fd7a:115c:a1e0::x`. Loopback and WSL's switch's
private ranges print as they are: the proofs' evidence rests on them, and they identify nothing. Only what is printed is
masked; every proof decides on the real addresses.

## References
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-912 — Windows containment, RQ-751: option (b) now, the Job Object proof passing with its two process-visibility gaps accepted and the job-queue mutants staying on GitHub's runners; option (c), CPU tests and mutants in a WSL2 distro under `contain.sh`, built in parallel and never blocking, replaces (b) only once each of its five requirements is proven, the job-queue mutants then free to move to Windows; GPU tests stay native *(closes RQ-751; amends R-905)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `.github/scripts/wsl.ps1`: `$TailnetV4` and `$TailnetV6`, the two patterns, and `Hide-Tailnet`, which applies them
  with their replacements (`-creplace`, the patterns as written); `Report` masks its line before printing it and
  keeping it; `Fail` and `Invoke-Wsl`'s echo of stdout and stderr write through it; the proofs' one other print that
  carries a message from outside (the clone's failure) too. Nothing a proof decides on is masked.
- The sweep of the other scripts the self-hosted jobs run: `scripts/wsl/inside.sh` prints addresses only to wsl.ps1,
  which reads them raw for proof 3 and masks its echo; a comment says so, and nothing in it changes.
  `win_contain.ps1`, `win_measure.ps1` and `git_guard.ps1` print no host address (loopback markers only), so are
  unchanged.
- `xtask/tests/wsl_mask.rs` (`regex`, already in the build, as xtask's dev-dependency): the patterns run on made-up
  tailnet, LAN, loopback, WSL-switch and IPv6 inputs; every console write, `Report` and proof 3's address variables
  through the mask; no other print of an address in the proofs; no masked value feeding a decision. Each with its
  negative controls.
- `xtask/tests/wsl.rs`'s `check_fail`: `Fail`'s body as it now reads, its control's edit with it.

## Acceptance tests
- `cargo nextest run -p xtask --test wsl_mask --test wsl --test ci_wsl_cache --test qa_wsl_proofs_scripts --test
  qa_win_wsl_cache --test qa_ci_win_download_retry`, with and without `--features controls`; `cargo clippy --workspace
  --all-targets -- -D warnings` with and without `--features controls`; `cargo fmt --check` (REQ-TOOL-240).
- The head pushed to `measure/<PR>-xtask-ci` as well, its xtask-ci run green (R-681) (REQ-TOOL-240).
- On the PC, a `wsl-proofs` dispatch after merge whose proof 3 line shows no tailnet address, its run id reported in the
  PR or a follow-up comment, a measurement (R-711) (REQ-TOOL-240).

## Notes
- Entered 14 Nov 2026 (applied per R-369: CI tooling within the runner rules, two-way; the owner's goal of 12 Nov 2026
  for the Windows runner).
- No real address appears in the tests, the PR or any commit: the tests' addresses are made up.
- REQ-TOOL-240: 238 is TASK-M0-66's, open at the same time; 240 leaves room.
