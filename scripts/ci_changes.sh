#!/usr/bin/env bash
# What a CI run's change touches, for the in-job skips R-656 allows: a docs-only pull request skips the build, test and
# GPU jobs, and one that changes no GPU-path file skips the GPU suites. Every required check still reports, under its
# own name, and concludes success (R-656; R-266, R-305).
#
#   scripts/ci_changes.sh detect       In a workflow step: classify this run's change and write `docs_only=…` and
#                                      `gpu=…` to $GITHUB_OUTPUT (and print them, with the changed paths).
#   scripts/ci_changes.sh classify     Read changed paths from stdin, one per line; print `docs_only=…` and `gpu=…`.
#   scripts/ci_changes.sh doc-tests    Print the nextest arguments of the tests that read the docs-only paths, one
#                                      package a line (`-p <package> [--lib --bins] [--test <target>]…`).
#
# The change is the pull request's merge commit against its first parent, the base it is merged onto: exactly what the
# pull request adds to that base. Any other event (a push to `main`, a `measure/` branch, a dispatch) is no pull
# request, and runs everything. Conservative throughout: a path counts as docs-only only if the allowlist below names
# it, and every path the GPU list doesn't exempt runs the GPU suites; `.github/` is neither, so a workflow change runs
# everything. A diff that can't be computed, or is empty, runs everything too, with a warning.
set -euo pipefail

# The docs-only allowlist: the corpus, the plan's prose and data, and the root's markdown. `plan/`'s Python (the plan
# checks) is code, so it is not on the list: a change to it runs everything.
is_docs() {
  case "$1" in
    docs/*) return 0 ;;
    plan/*.py | plan/tools/*) return 1 ;;
    plan/*.md | plan/*.yaml | plan/tasks/* | plan/reviewers/*) return 0 ;;
    decisions.md | REVIEW_QUEUE.md | open-questions.md | README.md | CLAUDE.md | AGENTS.md | GEMINI.md) return 0 ;;
  esac
  return 1
}

# The paths no GPU job builds, reads or runs: the docs-only ones, and the two crates nothing in the GPU jobs depends
# on (`gui` and `prin`; the GPU jobs build validation, engine, render, kernel, ledger and xtask). Everything else, a
# lockfile, a toolchain, a fixture or a workflow among it, runs the GPU suites.
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

# A line of Rust that names a docs-only path, outside a `//` comment: a test or source reading the docs.
DOC_TOKENS='(docs|plan)/|decisions\.md|REVIEW_QUEUE|open-questions|CLAUDE\.md|AGENTS\.md|GEMINI\.md|README|"docs"|"plan"'
# Not `grep -q`: it exits at its first match, and the `grep -v` feeding it then dies of SIGPIPE, which pipefail reports
# as a failure.
names_docs() {
  grep -vE '^[[:space:]]*//' "$1" | grep -E "$DOC_TOKENS" > /dev/null
}

# Every package's tests that read a docs-only path: an integration test target (a top-level `tests/*.rs`) whose source
# names one, or that names a support file (a `tests/<dir>/…` file, any crate's) that does; and the library and binary
# unit tests of a package whose `src/` names one. Over-selects rather than misses: a mention is enough.
doc_tests() {
  local pkg_dir pkg args f target base
  local supports=""
  for f in crates/*/tests/*/*.rs crates/*/tests/*/*/*.rs xtask/tests/*/*.rs xtask/tests/*/*/*.rs; do
    [ -f "$f" ] || continue
    names_docs "$f" && supports="$supports $(basename "$f")"
  done
  for pkg_dir in crates/*/ xtask/; do
    [ -f "$pkg_dir/Cargo.toml" ] || continue
    pkg=$(basename "$pkg_dir")
    args=""
    for f in $(find "${pkg_dir}src" -name '*.rs' 2> /dev/null | sort); do
      if names_docs "$f"; then
        args=" --lib --bins"
        break
      fi
    done
    for f in "${pkg_dir}"tests/*.rs; do
      [ -f "$f" ] || continue
      target=$(basename "$f" .rs)
      if names_docs "$f"; then
        args="$args --test $target"
        continue
      fi
      for base in $supports; do
        if grep -qF "$base" "$f"; then
          args="$args --test $target"
          break
        fi
      done
    done
    [ -z "$args" ] || echo "-p $pkg$args"
  done
}

case "${1:-}" in
  detect) detect ;;
  classify) classify ;;
  doc-tests) cd "$(git rev-parse --show-toplevel)" && doc_tests ;;
  *)
    echo "usage: scripts/ci_changes.sh detect | classify | doc-tests" >&2
    exit 2
    ;;
esac
