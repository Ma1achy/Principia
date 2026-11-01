#!/usr/bin/env bash
# What a CI run's change touches, for the in-job skips R-656 allows and R-662 scopes: full CI runs only when a pull
# request changes Principia's code (the owner, 31 Oct 2026: "Only the actual Principia CODE matters in ci"). One that
# changes none of it runs only the cheap checks, plan-check, the vocabulary lint and the atlas guard, and one that
# changes no GPU-path file skips the GPU suites. Every required check still reports, under its own name, and concludes
# success (R-656; R-266, R-305).
#
#   scripts/ci_changes.sh detect       In a workflow step: classify this run's change and write `docs_only=…` and
#                                      `gpu=…` to $GITHUB_OUTPUT (and print them, with the changed paths).
#   scripts/ci_changes.sh classify     Read changed paths from stdin, one per line; print `docs_only=…` and `gpu=…`.
#
# `docs_only` keeps its name from R-656 and now reads "changes no Principia code": scripts and the plan's Python are on
# the same side as the docs (R-662).
#
# The change is the pull request's merge commit against its first parent, the base it is merged onto: exactly what the
# pull request adds to that base. Any other event (a push to `main`, a `measure/` branch, a dispatch) is no pull
# request, and runs everything. It fails towards full CI: a path counts as docs-only only if the allowlist below names
# it and the list of data code loads does not, so an unknown top-level path, `.github/`, `.cargo/` or a new folder runs
# everything, and every path the GPU list doesn't exempt runs the GPU suites. A mixed change runs everything it would
# for its code. A diff that can't be computed, or is empty, runs everything too, with a warning.
set -euo pipefail

# The allowlist of what is not Principia's code: the corpus and its papers, the plan's prose and Python, the scripts,
# the workbench and the root's markdown (R-662).
on_allowlist() {
  case "$1" in
    docs/* | plan/* | scripts/* | workbench/*) return 0 ;;
    decisions.md | REVIEW_QUEUE.md | open-questions.md | README.md | CLAUDE.md | AGENTS.md | GEMINI.md) return 0 ;;
  esac
  return 1
}

# The files on that allowlist that are data code loads for behaviour, or CI's own machinery, which count as code
# (R-662, narrowing R-656's list of every file any test reads). Each pattern names its readers. A `case` pattern's `*`
# matches `/` too. `xtask/tests/ci_changes.rs` checks that each pattern classifies as code, and scans the sources for a
# data file they name that classifies as docs-only.
reads_code() {
  case "$1" in
    # The artboards: `cargo xtask screenshot` through the cases' `artboard` paths (fixtures/screenshot/*/cases.json).
    docs/gui/design/*) return 0 ;;
    # The references the goldens are compared with: xtask/src/compass_golden.rs (principia_nav_cube.html); the
    # print-graph catalogue, crates/kernel/tests/qa_TASK-M2-08.rs (ic_inspector.html) and
    # crates/render/tests/qa_TASK-M1-03.rs (principia_colour_explorer.html).
    docs/gui/reference/*) return 0 ;;
    # A gate's thresholds are refused unless they name a live requirement of the right kind
    # (crates/validation/src/gate/mod.rs, `config`, through `check_requirement`); `cargo xtask gate-report` reads each
    # requirement's `verify` (xtask/src/gate_report.rs).
    plan/requirements.yaml) return 0 ;;
    # `cargo xtask gate-report`: which task closes a review-checklist requirement (xtask/src/gate_report.rs).
    plan/tasks.yaml) return 0 ;;
    # `cargo xtask gate-report`: the ids in each gate block (xtask/src/gate_report.rs, `gate_ids`).
    plan/MILESTONES.md) return 0 ;;
    # `cargo xtask reviews-check`: the `Reviewers` field decides `reviews-complete` (xtask/src/reviews_check.rs).
    plan/tasks/*) return 0 ;;
    # This classifier: what it says decides what every pull request runs (`ci.yml`, `mutants.yml`).
    scripts/ci_changes.sh) return 0 ;;
  esac
  return 1
}

# Docs-only: on the allowlist, and neither data code loads nor CI's machinery.
is_docs() {
  on_allowlist "$1" && ! reads_code "$1"
}

# The paths no GPU job builds, reads or runs: the docs-only ones, and the two crates nothing in the GPU jobs depends
# on (`gui` and `prin`; the GPU jobs build validation, engine, render, kernel, ledger and xtask). Everything else, a
# lockfile, a toolchain, a fixture, a workflow or data code loads among it, runs the GPU suites.
is_not_gpu() {
  is_docs "$1" && return 0
  case "$1" in
    crates/gui/* | crates/prin/*) return 0 ;;
  esac
  return 1
}

# Reads paths on stdin; prints the verdict. No path at all runs everything.
classify() {
  local docs_only=true gpu=false n=0 path
  while IFS= read -r path || [ -n "$path" ]; do
    [ -n "$path" ] || continue
    n=$((n + 1))
    is_docs "$path" || docs_only=false
    is_not_gpu "$path" || gpu=true
  done
  if [ "$n" -eq 0 ]; then
    docs_only=false
    gpu=true
  fi
  echo "docs_only=$docs_only"
  echo "gpu=$gpu"
}

# The paths the pull request changes, against the base its merge commit is merged onto. Fetches the merge commit's
# parents (the checkout is one commit deep); prints nothing if anything fails, which classify reads as "run all".
changed_paths() {
  local head="${GITHUB_SHA:?GITHUB_SHA is not set}"
  if ! git rev-parse --verify --quiet "$head^1" > /dev/null; then
    git fetch --no-tags --quiet --depth=2 origin "$head" > /dev/null 2>&1 || return 0
  fi
  git rev-parse --verify --quiet "$head^2" > /dev/null || return 0
  git -c core.quotePath=false diff --name-only --no-renames "$head^1" "$head" || true
}

detect() {
  local verdict paths
  if [ "${GITHUB_EVENT_NAME:-}" = pull_request ]; then
    paths=$(changed_paths)
    if [ -z "$paths" ]; then
      echo "::warning::ci_changes: no changed path found for $GITHUB_SHA; running everything"
    else
      echo "Changed paths ($(printf '%s\n' "$paths" | wc -l | tr -d ' ')):"
      printf '%s\n' "$paths" | sed 's/^/  /'
    fi
  else
    paths=""
    echo "${GITHUB_EVENT_NAME:-no event}: not a pull request; running everything"
  fi
  verdict=$(printf '%s' "$paths" | classify)
  echo "$verdict"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "$verdict" >> "$GITHUB_OUTPUT"
  fi
}

case "${1:-}" in
  detect) detect ;;
  classify) classify ;;
  *)
    echo "usage: scripts/ci_changes.sh detect | classify" >&2
    exit 2
    ;;
esac
