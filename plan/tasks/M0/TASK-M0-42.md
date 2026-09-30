# TASK-M0-42 — CI caches only the cargo registry and the fixture pool, with per-job keys

- **Milestone:** M0
- **Closes:** REQ-SYS-073
- **Depends on:** TASK-M0-33
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~80 lines

## Goal
Every workflow uses `Swatinem/rust-cache`, which caches `target` by default. With the fixture pool (R-270) added, the
repository's Actions cache sits near GitHub's 10 GB limit, so evictions make cold runs more common (qa on PR #74).
CI caches only the cargo registry and the fixture pool, never whole target directories, each under a key naming its
job, so the cache stays well under the limit (R-285).

## References
- `decisions.md` § "R-285 — CI caches only the cargo registry and the fixture pool, with per-job keys"
- `decisions.md` § "R-270 — TASK-M0-33: qa's one-round exception is granted; the fixture-pool cost is sent back *(amends R-231)*"

## Deliverables
- `.github/workflows/*.yml`: no step caches a target directory (for `Swatinem/rust-cache`, turn off its target cache, or replace it with an `actions/cache` step on the cargo registry and git directories). The fixture-pool cache stays, saved only by the job that builds it.
- Every cache key names its job.
- The PR shows the Actions cache listing (`gh cache list`: keys, sizes and the total) after one run of every workflow, and the `ci` job's wall time before and after.

## Acceptance tests
- Review (code, qa): no workflow caches a target directory, every key names its job, and the cache total the PR shows is well under 10 GB (REQ-SYS-073).
- CI log on the PR head: the `ci` job's wall time is no slower than R-270's ~10.5 min (REQ-SYS-073).

## Notes
- A slower `ci` job than R-270's target goes back to the human as a cost, not a waiver.
