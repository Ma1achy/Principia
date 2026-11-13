# Measured times of the Windows runner's jobs and steps (R-988, REQ-TOOL-232)

Every job on R-711's runner, and every step of one that builds or runs tests, carries `timeout-minutes` of 1.5 times
its measured time, rounded up to a whole minute (R-988). This table holds those measured times with the runs they come
from, committed before any limit was set (R-649); `xtask/tests/ci_self_hosted.rs` (`self_hosted_jobs_have_time_limits`)
reads it and fails if a limit in `win-gpu.yml` or `runner-proof.yml` is not the one its row gives.

The criterion, written on 13 Nov 2026 before any job's or step's time was read (applied per R-369, CI timing):

- **Which jobs:** every job of `win-gpu.yml` and `runner-proof.yml` that runs on the runner (not plainly a hosted image);
  `wsl-image` and `wsl-parity` run on GitHub's runners and are not listed.
- **Which steps:** every step that builds or runs tests, read as every step that starts its command through
  `.github/scripts/win_measure.ps1 run` (each job's recorded work: the cargo builds, the suites, the gates, the
  containment check, the proofs, `wsl-cpu`'s tests and the registration).
- **A completed run of a step:** it concluded `success` or `failure`, never skipped, cancelled or timed out, and ran
  its work through: a step that refused before its work (the registration's exit 125) did not (this clause made
  explicit on review, before any limit acted; see the note under the table).
- **A completed run of a job:** it concluded `success` or `failure`, and every step of it listed here was a completed
  run; for a job with no such step (`runner-proof`), its conclusion alone. A job that failed early, its suites skipped,
  did not run its work through.
- **Measured time:** the slowest of its last five completed runs on the PC (by start time), or all if fewer, in whole
  seconds from the jobs API's `started_at` and `completed_at`, every attempt of every run of either workflow read.
- **Limit:** the ceiling of 1.5 times the measured time, in minutes. A job or step with no completed run on record keeps
  its present limit until it has one (R-988): the row says "none" for its runs, and its limit is the one the workflow
  had, "none" where a step had none and its job's limit bounds it.

Read on 13 Nov 2026 from `gh api repos/Ma1achy/Principia/actions/runs/<run>/attempts/<n>/jobs`, over every run of
`win-gpu.yml` (86 listed, the last created 12 Nov 2026 23:53 UTC) and `runner-proof.yml` (one).

| Workflow | Job | Step | Completed runs on the PC, newest first (run id, job id, seconds) | Measured (s) | Limit (min) |
| --- | --- | --- | --- | --- | --- |
| win-gpu.yml | win-vulkan | (job) | run 43316207795, job 131524774857, 4077 s; run 43288884452, job 131429514850, 4144 s; run 43243728779, job 131270707151, 4142 s; run 43230901521, job 131225617401, 6203 s; run 43224843606, job 131205264773, 4086 s | 6203 | 156 |
| win-gpu.yml | win-vulkan | cargo xtask build-kernel | run 43316207795, job 131524774857, 90 s; run 43288884452, job 131429514850, 51 s; run 43243728779, job 131270707151, 79 s; run 43230901521, job 131225617401, 56 s; run 43224843606, job 131205264773, 70 s | 90 | 3 |
| win-gpu.yml | win-vulkan | The harness runs on the RTX 2080 SUPER | run 43316207795, job 131524774857, 81 s; run 43288884452, job 131429514850, 77 s; run 43243728779, job 131270707151, 82 s; run 43230901521, job 131225617401, 451 s; run 43224843606, job 131205264773, 74 s | 451 | 12 |
| win-gpu.yml | win-vulkan | cargo nextest run -p validation -p xtask -p engine | run 43316207795, job 131524774857, 701 s; run 43288884452, job 131429514850, 728 s; run 43243728779, job 131270707151, 745 s; run 43230901521, job 131225617401, 1745 s; run 43224843606, job 131205264773, 698 s | 1745 | 44 |
| win-gpu.yml | win-vulkan | cargo test -p validation -p xtask -p engine --doc | run 43316207795, job 131524774857, 2 s; run 43288884452, job 131429514850, 2 s; run 43243728779, job 131270707151, 2 s; run 43230901521, job 131225617401, 3 s; run 43224843606, job 131205264773, 3 s | 3 | 1 |
| win-gpu.yml | win-vulkan | cargo nextest run -p validation --features controls gpu_harness | run 43316207795, job 131524774857, 14 s; run 43288884452, job 131429514850, 11 s; run 43243728779, job 131270707151, 12 s; run 43230901521, job 131225617401, 13 s; run 43224843606, job 131205264773, 13 s | 14 | 1 |
| win-gpu.yml | win-vulkan | cargo nextest run -p validation --features controls compute_fast_math | run 43316207795, job 131524774857, 10 s; run 43288884452, job 131429514850, 10 s; run 43243728779, job 131270707151, 9 s; run 43230901521, job 131225617401, 9 s; run 43224843606, job 131205264773, 9 s | 10 | 1 |
| win-gpu.yml | win-vulkan | cargo nextest run -p engine --features controls compute_entry | run 43316207795, job 131524774857, 57 s; run 43288884452, job 131429514850, 57 s; run 43243728779, job 131270707151, 61 s; run 43230901521, job 131225617401, 306 s; run 43224843606, job 131205264773, 69 s | 306 | 8 |
| win-gpu.yml | win-vulkan | cargo nextest run -p engine --features controls decode_ | run 43316207795, job 131524774857, 854 s; run 43288884452, job 131429514850, 862 s; run 43243728779, job 131270707151, 863 s; run 43230901521, job 131225617401, 872 s; run 43224843606, job 131205264773, 871 s | 872 | 22 |
| win-gpu.yml | win-vulkan | cargo nextest run -p engine --features controls invariant_ | run 43316207795, job 131524774857, 46 s; run 43288884452, job 131429514850, 42 s; run 43243728779, job 131270707151, 42 s; run 43230901521, job 131225617401, 42 s; run 43224843606, job 131205264773, 42 s | 46 | 2 |
| win-gpu.yml | win-vulkan | cargo nextest run -p engine --features controls qa_m206_ | run 43316207795, job 131524774857, 195 s; run 43288884452, job 131429514850, 197 s; run 43243728779, job 131270707151, 198 s; run 43230901521, job 131225617401, 198 s; run 43224843606, job 131205264773, 198 s | 198 | 5 |
| win-gpu.yml | win-vulkan | cargo nextest run -p engine --features controls canonicaliser_ | run 43316207795, job 131524774857, 515 s; run 43288884452, job 131429514850, 521 s; run 43243728779, job 131270707151, 517 s; run 43230901521, job 131225617401, 516 s; run 43224843606, job 131205264773, 517 s | 521 | 14 |
| win-gpu.yml | win-vulkan | cargo nextest run -p engine --features controls encode_ | run 43316207795, job 131524774857, 124 s; run 43288884452, job 131429514850, 121 s; run 43243728779, job 131270707151, 119 s; run 43230901521, job 131225617401, 119 s; run 43224843606, job 131205264773, 120 s | 124 | 4 |
| win-gpu.yml | win-vulkan | cargo xtask gate decode-agreement | run 43316207795, job 131524774857, 1001 s; run 43288884452, job 131429514850, 1019 s; run 43243728779, job 131270707151, 1006 s; run 43230901521, job 131225617401, 1001 s; run 43224843606, job 131205264773, 1003 s | 1019 | 26 |
| win-gpu.yml | win-vulkan | cargo nextest run -p validation --features controls codegen_selftest | run 43316207795, job 131524774857, 77 s; run 43288884452, job 131429514850, 73 s; run 43243728779, job 131270707151, 76 s; run 43230901521, job 131225617401, 75 s; run 43224843606, job 131205264773, 77 s | 77 | 2 |
| win-gpu.yml | win-vulkan | cargo xtask golden --all | run 43316207795, job 131524774857, 48 s; run 43288884452, job 131429514850, 50 s; run 43243728779, job 131270707151, 46 s; run 43230901521, job 131225617401, 46 s; run 43224843606, job 131205264773, 47 s | 50 | 2 |
| win-gpu.yml | win-vulkan | cargo nextest run -p xtask --features controls golden_halfway | run 43316207795, job 131524774857, 132 s; run 43288884452, job 131429514850, 115 s; run 43243728779, job 131270707151, 119 s; run 43230901521, job 131225617401, 580 s; run 43224843606, job 131205264773, 116 s | 580 | 15 |
| win-gpu.yml | win-tests | (job) | run 43230901521, job 131225616994, 5863 s; run 42825370464, job 129861170030, 5600 s; run 42630146970, job 129204041715, 4129 s | 5863 | 147 |
| win-gpu.yml | win-tests | cargo build --workspace --all-targets | run 43230901521, job 131225616994, 3274 s; run 43023140554, job 130531059795, 751 s; run 42825370464, job 129861170030, 3036 s; run 42630146970, job 129204041715, 599 s | 3274 | 82 |
| win-gpu.yml | win-tests | cargo nextest run --workspace | run 43230901521, job 131225616994, 1724 s; run 42825370464, job 129861170030, 1457 s; run 42630146970, job 129204041715, 1659 s | 1724 | 44 |
| win-gpu.yml | win-tests | cargo nextest run -p validation --test qa_TASK-M0-24 | run 43230901521, job 131225616994, 6 s; run 42825370464, job 129861170030, 7 s; run 42630146970, job 129204041715, 1555 s | 1555 | 39 |
| win-gpu.yml | win-tests | cargo nextest run -p xtask --test qa_TASK-M0-06_list | run 43230901521, job 131225616994, 573 s; run 42825370464, job 129861170030, 811 s; run 42630146970, job 129204041715, 126 s | 811 | 21 |
| win-gpu.yml | win-tests | cargo test --workspace --doc | run 43230901521, job 131225616994, 5 s; run 42825370464, job 129861170030, 3 s; run 42630146970, job 129204041715, 5 s | 5 | 1 |
| win-gpu.yml | win-tests | cargo xtask deps | run 43230901521, job 131225616994, 109 s; run 42825370464, job 129861170030, 99 s; run 42630146970, job 129204041715, 69 s | 109 | 3 |
| win-gpu.yml | win-self-check | (job) | run 43230901521, job 131225617629, 393 s; run 43023140554, job 130531059678, 448 s; run 42825370464, job 129861170106, 355 s; run 42630146970, job 129204041981, 332 s; run 42520229425, job 128851914552, 649 s | 649 | 17 |
| win-gpu.yml | win-self-check | The guard on this workflow | run 43230901521, job 131225617629, 177 s; run 43023140554, job 130531059678, 166 s; run 42825370464, job 129861170106, 157 s; run 42630146970, job 129204041981, 148 s; run 42520229425, job 128851914552, 173 s | 177 | 5 |
| win-gpu.yml | win-contain | (job) | none | — | 30 |
| win-gpu.yml | win-contain | Containment check | none | — | none |
| win-gpu.yml | wsl-register | (job) | none | — | 95 |
| win-gpu.yml | wsl-register | Register principia-wsl | none | — | none |
| win-gpu.yml | wsl-proofs | (job) | none | — | 180 |
| win-gpu.yml | wsl-proofs | The six proofs | none | — | none |
| win-gpu.yml | wsl-cpu | (job) | none | — | 330 |
| win-gpu.yml | wsl-cpu | CPU tests and the job queue's mutants in WSL | none | — | none |
| runner-proof.yml | runner-proof | (job) | run 42476517881, job 128718974418, 66 s | 66 | 2 |

No completed run of `wsl-register`, its present limit kept. The job's work is the registration, and no run on the PC
has done it: its three runs that concluded `success` or `failure` with the step run (run 43128608743, job
130895892272, 2017 s; run 42954587933, job 130313726220, 1239 s; run 42944409475, job 130277996649, 1192 s) each
refused within 2 s, exit 125, no image pinned or one that didn't match (2 s, 0 s and 1 s), and no other run reached
the step, run 43225140437's attempts 2 (3182 s) and 3 (2273 s) among them. A refusal before the work is not a run of
it, so neither the job nor its step "Register principia-wsl" has a completed run on record, and each keeps its present
limit until it has one (R-988): the job its 95 minutes, the step none of its own. This reading of the criterion was
corrected on review, before any limit acted (applied per R-369); the criterion above says the same of a job that failed
early.

Rows read from failed runs alone. Every recorded run of these steps failed on its own output, so its time may be
that of the failure, not of the work, and the limit could cut the step once it passes: `compute_fast_math`,
`golden --all` and `golden_halfway` of `win-vulkan`. Each ran its tests and failed on their output, so its time is
the work's; the criterion takes them as they are. They are named here so that a cut of one is read as the limit's, not
the step's, and is put to the owner rather than met by a limit changed after the result (R-649).
