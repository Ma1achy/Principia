#!/usr/bin/env bash
# What a CI run's change touches, for the in-job skips R-656 allows, R-662 scopes and R-677 splits into three tiers:
#   (a) code: full CI runs when a pull request changes Principia's code (the owner, 31 Oct 2026: "Only the actual
#       Principia CODE matters in ci"), or data code loads that only full CI's jobs read;
#   (b) data: a change to the plan's task data (`plan/tasks/`, `plan/tasks.yaml`, `plan/MILESTONES.md`, and
#       `plan/requirements.yaml` when only fields no code reads change, as `scripts/ci_req_fields.py` decides) or the
#       GUI's artboards and references (`docs/gui/design/`, `docs/gui/reference/`) runs only the cheap checks and the
#       tests that read what it changed (R-677; `ci.yml`'s `ci-workspace`): no build, no test shard, no GPU suite, no
#       mutants;
#   (c) docs: the rest of the corpus, the plan's prose and Python, the scripts, the workbench and `.claude/` run only
#       the cheap checks, plan-check, the vocabulary lint and the atlas guard (R-662, R-790).
# A change touching several tiers runs the highest. One that touches no `.rs` file under `crates/` has no mutant to
# run (R-677). Every required check still reports, under its own name, and concludes success only where its tier allows
# it (R-656; R-266, R-305).
#
#   scripts/ci_changes.sh detect       In a workflow step: classify this run's change and write its verdict to
#                                      $GITHUB_OUTPUT (and print it, with the changed paths).
#   scripts/ci_changes.sh classify     Read changed paths from stdin, one per line; print the verdict. With
#                                      CI_REQS_BASE and CI_REQS_HEAD naming the base's and the head's copies of
#                                      `plan/requirements.yaml`, that file is tier (b) when `scripts/ci_req_fields.py`
#                                      (or CI_REQ_FIELDS, naming it) says so; without them it is code.
#
# The verdict, one `key=value` per line:
#   tier=code|data|docs   the highest tier of any path;
#   docs_only=true|false  true when the tier is data or docs: no full CI. The name is R-656's, kept so the jobs' skips,
#                         the gates and R-335's `if:` stand; R-662 read it "changes no Principia code", R-677 reads it
#                         "runs no full CI";
#   gpu=true|false        false when no path is a GPU path, as on every data or docs change;
#   plan_data=true|false  some path is the plan's task data (tier b), `plan/requirements.yaml` among it when tier (b);
#   gui_data=true|false   some path is the GUI's artboards or references (tier b);
#   mutants=true|false    some path is a `.rs` file under `crates/`, so the merge-ready head's mutants run (R-677).
#
# The change is the pull request's merge commit against its first parent, the base it is merged onto: exactly what the
# pull request adds to that base. Any other event (a push to `main`, a `measure/` branch, a dispatch) is no pull
# request, and runs everything. It fails towards full CI: a path is data or docs only if the allowlist below names it
# and the list of data only full CI reads does not, so an unknown top-level path, `.github/`, `.cargo/` or a new folder
# runs everything, and every code path the GPU list doesn't exempt runs the GPU suites. A diff that can't be computed,
# or is empty, runs everything too, its mutants among it, with a warning. `plan/requirements.yaml` is code unless
# `scripts/ci_req_fields.py` finds both copies and says only fields no code reads changed (R-677): an added or removed
# requirement, a field code reads, an unknown field, a parse error or a missing copy runs everything.
set -euo pipefail

# Whether this change's `plan/requirements.yaml` is tier (b): set once by `classify`, from CI_REQS_BASE and
# CI_REQS_HEAD. The field check prints its one reason line on stderr, so the job's log shows why.
reqs_data=false
requirements_check() {
  local fields="${CI_REQ_FIELDS:-$(dirname "${BASH_SOURCE[0]}")/ci_req_fields.py}"
  reqs_data=false
  if [ -n "${CI_REQS_BASE:-}" ] && [ -n "${CI_REQS_HEAD:-}" ] \
    && python3 -I "$fields" "$CI_REQS_BASE" "$CI_REQS_HEAD" >&2; then
    reqs_data=true
  fi
}

# Whether the path is `plan/requirements.yaml` with only fields no code reads changed (R-677).
requirements_data() {
  [ "$1" = plan/requirements.yaml ] && [ "$reqs_data" = true ]
}

# The allowlist of what is not Principia's code: the corpus and its papers, the plan's prose, data and Python, the
# scripts, the workbench and the root's markdown (R-662), and `.claude/`, its commands, agents and settings (R-790).
on_allowlist() {
  case "$1" in
    docs/* | plan/* | scripts/* | workbench/* | .claude/*) return 0 ;;
    decisions.md | REVIEW_QUEUE.md | open-questions.md | README.md | CLAUDE.md | AGENTS.md | GEMINI.md) return 0 ;;
  esac
  return 1
}

# The files on that allowlist that are data code loads for behaviour, or scripts CI's and the tests' runs go through,
# which only full CI's jobs read, so they count as code (R-662, narrowing R-656's list of every file any test reads;
# R-677 moving the rest to tier b). Each pattern names its readers. A `case` pattern's `*` matches `/` too.
# `xtask/tests/ci_changes.rs` checks that each pattern classifies as code, and scans the sources for a data file they
# name that classifies as docs.
reads_code() {
  case "$1" in
    # Every numerical gate refuses a threshold that names no live requirement of the right kind
    # (crates/validation/src/gate/mod.rs, `config`, through `check_requirement`): the GPU suites' `cargo xtask gate
    # decode-agreement` and the gpu-kernel profile's crates/validation/tests/decode_f32_eps_gpu.rs and
    # invariant_gradient_gpu.rs among them. The constants register's calibration citations
    # (crates/ledger/tests/qa_TASK-M3-06_tau.rs) and validation's gate tests read it too. Tier (b) builds no kernel and
    # runs no GPU suite, so it is code (R-677, applied per R-369), unless `scripts/ci_req_fields.py` says only fields
    # none of these read changed (`requirements_data`, which `tier_of` asks first).
    plan/requirements.yaml) return 0 ;;
    # The field check that decides that.
    scripts/ci_req_fields.py) return 0 ;;
    # This classifier: what it says decides what every pull request runs (`ci.yml`, `mutants.yml`).
    scripts/ci_changes.sh) return 0 ;;
    # The containment: the `shell:` of the mutants steps (`mutants.yml`, `nightly.yml`), and nextest's wrapper of the
    # job queue's tests (.config/nextest.toml), so every run of them goes through it.
    scripts/contain.sh) return 0 ;;
  esac
  return 1
}

# Tier (b), the plan's task data, which only xtask's gate-report, reviews-check, plan-check and progress banner and
# validation's Part 6 catalogue read: `ci-workspace` runs those readers' tests on it (R-677). Each pattern names its
# readers. `xtask/tests/ci_changes.rs` checks that every reader the sources show is among the tests that step runs.
plan_data() {
  case "$1" in
    # `cargo xtask reviews-check`: the `Reviewers` field decides `reviews-complete` (xtask/src/reviews_check.rs); every
    # task file's title and roles (xtask/tests/qa_TASK-M0-36.rs); the plan checks' own tests; the print-graph coverage
    # (xtask/tests/qa_R-485_print_graph_coverage.rs) and the Part 6 catalogue
    # (crates/validation/tests/part6_catalogue.rs), each walking the task files.
    plan/tasks/*) return 0 ;;
    # `cargo xtask gate-report`: which task closes a review-checklist requirement (xtask/src/gate_report.rs); the
    # progress banner's tests (xtask/tests/progress_banner.rs).
    plan/tasks.yaml) return 0 ;;
    # `cargo xtask gate-report` and `gate-provisional`: the ids in each gate block (xtask/src/gate_report.rs,
    # `gate_ids`).
    plan/MILESTONES.md) return 0 ;;
  esac
  return 1
}

# Tier (b), the GUI's artboards and references, which only the screenshot tests and the tests comparing with a
# reference read: `ci-workspace` runs those tests on it (R-677), and `screenshot.yml` runs every screenshot suite on a
# pull request touching `docs/gui/`.
gui_data() {
  case "$1" in
    # The artboards: `cargo xtask screenshot` through the cases' `artboard` paths (fixtures/screenshot/*/cases.json);
    # the screenshot tests (xtask/tests/screenshot.rs, xtask/tests/qa_TASK-M6-24.rs).
    docs/gui/design/*) return 0 ;;
    # The references the tests compare with: the print-graph catalogue (xtask/tests/qa_R-485_print_graph_coverage.rs),
    # crates/kernel/tests/qa_TASK-M2-08.rs (ic_inspector.html) and crates/render/tests/qa_TASK-M1-03.rs
    # (principia_colour_explorer.html). The compass golden compares with records made from principia_nav_cube.html,
    # which are fixtures (fixtures/screenshot/01_main/compass/reference/), and so code.
    docs/gui/reference/*) return 0 ;;
  esac
  return 1
}

# A path's tier: code, data or docs.
tier_of() {
  if requirements_data "$1"; then
    echo data
  elif ! on_allowlist "$1" || reads_code "$1"; then
    echo code
  elif plan_data "$1" || gui_data "$1"; then
    echo data
  else
    echo docs
  fi
}

# The paths no GPU job builds, reads or runs: the data and docs ones, and the two crates nothing in the GPU jobs depends
# on (`gui` and `prin`; the GPU jobs build validation, engine, render, kernel, ledger and xtask). Everything else, a
# lockfile, a toolchain, a fixture, a workflow or data only full CI reads among it, runs the GPU suites.
is_not_gpu() {
  if [ "$(tier_of "$1")" != code ]; then
    return 0
  fi
  case "$1" in
    crates/gui/* | crates/prin/*) return 0 ;;
  esac
  return 1
}

# Whether a path is Rust source under `crates/`, the only code the merge-ready head's mutants run on (R-677).
has_mutants() {
  case "$1" in
    crates/*.rs) return 0 ;;
  esac
  return 1
}

# Reads paths on stdin; prints the verdict. No path at all runs everything.
classify() {
  local tier=docs gpu=false plan=false gui=false mutants=false n=0 path
  requirements_check
  while IFS= read -r path || [ -n "$path" ]; do
    [ -n "$path" ] || continue
    n=$((n + 1))
    case "$(tier_of "$path")" in
      code) tier=code ;;
      data) if [ "$tier" = docs ]; then tier=data; fi ;;
    esac
    if ! is_not_gpu "$path"; then gpu=true; fi
    if plan_data "$path"; then plan=true; fi
    if requirements_data "$path"; then plan=true; fi
    if gui_data "$path"; then gui=true; fi
    if has_mutants "$path"; then mutants=true; fi
  done
  if [ "$n" -eq 0 ]; then
    tier=code
    gpu=true
    mutants=true
  fi
  local docs_only=true
  if [ "$tier" = code ]; then docs_only=false; fi
  echo "tier=$tier"
  echo "docs_only=$docs_only"
  echo "gpu=$gpu"
  echo "plan_data=$plan"
  echo "gui_data=$gui"
  echo "mutants=$mutants"
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

# Writes the base's and the head's `plan/requirements.yaml` to a fresh folder and names them in CI_REQS_BASE and
# CI_REQS_HEAD, for `classify`'s field check. A copy git cannot show (a file added or deleted) is left unnamed, so the
# file stays code.
requirements_copies() {
  local head="${GITHUB_SHA:?GITHUB_SHA is not set}" dir
  dir=$(mktemp -d)
  if git show "$head^1:plan/requirements.yaml" > "$dir/base.yaml" 2> /dev/null \
    && git show "$head:plan/requirements.yaml" > "$dir/head.yaml" 2> /dev/null; then
    export CI_REQS_BASE="$dir/base.yaml" CI_REQS_HEAD="$dir/head.yaml"
  fi
}

detect() {
  local verdict paths
  unset CI_REQS_BASE CI_REQS_HEAD
  if [ "${GITHUB_EVENT_NAME:-}" = pull_request ]; then
    paths=$(changed_paths)
    if [ -z "$paths" ]; then
      echo "::warning::ci_changes: no changed path found for $GITHUB_SHA; running everything"
    else
      echo "Changed paths ($(printf '%s\n' "$paths" | wc -l | tr -d ' ')):"
      printf '%s\n' "$paths" | sed 's/^/  /'
    fi
    if printf '%s\n' "$paths" | grep -qx 'plan/requirements.yaml'; then
      requirements_copies
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
