#!/usr/bin/env bash
# CI's two code tiers (R-684): full CI on the merge-candidate head, the one carrying the `merge-ready` label, and on
# every push to `main`; the quick tier on every other pull-request head. R-677's classifier applies first: a docs or
# data head runs neither.
#
#   .github/scripts/ci_tier.sh decide        In `ci.yml`'s `changes` job, after `scripts/ci_changes.sh detect`: this run's
#                                            tier, written to $GITHUB_OUTPUT. On a code head each full-only required
#                                            check is posted pending.
#   .github/scripts/ci_tier.sh report        In `ci.yml`'s `ci-status` job: each full-only check's status, from its gate.
#   .github/scripts/ci_tier.sh merge-ready   In `ci-full.yml`: `merge-ready` put on starts the head's full tier, without
#                                            a push; taken off, it cancels a full attempt still going.
#
# How a head goes from quick to full. A label change must not start a run of `ci.yml`: of each workflow's runs on a head
# branch protection counts only the latest run's check suite (R-678), so another label's run would supersede the suite
# holding the head's required checks. So `ci.yml` runs on no label change. `ci-full.yml` re-runs the head's `pull_request`
# run instead, as `reviews.yml` re-runs its own (R-276): a new attempt of the same run, in the same check suite, on the
# same merge commit. The new attempt finds the label on and is full.
#
# Pending, never skipped-as-success. On a quick run no job bears a required name, so a skip cannot pass one: a gate
# skipped at job level reports its raw `name:` (`ci.yml`, the gates' comment), and until the full tier runs branch
# protection reads the check as expected. `decide` also posts each one pending as a commit status, which belongs to no
# check suite and so no later run supersedes. When a check and a commit status share a name, GitHub requires both, so
# `report` always closes what `decide` opened; that holds whether or not branch protection's app binding
# (`required_status_checks`, app 15368) counts a status at all.
set -euo pipefail

# The required checks only the full tier reports. `pr-check`, `reviews-complete` and `mutants-check` report on every
# head (R-678).
FULL_ONLY="ci xtask-ci gpu-kernel gpu-metal gpu-lavapipe"

here=$(dirname "${BASH_SOURCE[0]}")

output() { # output <key> <value>
  echo "$1=$2"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "$1=$2" >> "$GITHUB_OUTPUT"
  fi
}

post() { # post <context> <state> <description>
  gh api --method POST "repos/$GH_REPO/statuses/$HEAD_SHA" \
    -f "state=$2" -f "context=$1" -f "description=$3" \
    -f "target_url=${GITHUB_SERVER_URL:-https://github.com}/$GH_REPO/actions/runs/${GITHUB_RUN_ID:-0}" > /dev/null
}

live_labels() {
  gh api "repos/$GH_REPO/pulls/$PR" --jq '.labels[].name'
}

# Whether this run is the full tier. A push is quick on its first attempt whatever its payload says: a push takes
# `merge-ready` off (`mutants.yml`, `merge-ready-off`), and the payload lists the label as it was. Any other pull-request
# run reads the label as it is now: a re-run keeps its first attempt's payload, so only the live label tells.
full_tier() {
  [ "${GITHUB_EVENT_NAME:-}" = pull_request ] || return 0
  if [ "${ACTION:-}" = synchronize ] && [ "${GITHUB_RUN_ATTEMPT:-1}" = 1 ]; then
    return 1
  fi
  local labels
  labels=$(live_labels) || {
    echo "::error::ci_tier: could not read pull request $PR's labels"
    exit 1
  }
  grep -qx merge-ready <<< "$labels"
}

# The quick tier's packages, as cargo arguments: `--workspace` if the change cannot be read, which fails towards every
# test. Sets `packages`, `doc_packages`, those of them with a library, which `cargo test --doc` needs and which can be
# none, and `workspace_tests`.
quick_packages() {
  local changes="${CI_CHANGES:-$here/../../scripts/ci_changes.sh}" crates="" libraries="" paths crate
  local script="$here/ci_quick_crates.py" workspace="${CI_ROOT:-.}"
  paths=$("$changes" paths) \
    && crates=$(python3 -I "$script" --root "$workspace" <<< "$paths") \
    && libraries=$(python3 -I "$script" --root "$workspace" --library <<< "$paths") \
    || crates=""
  if [ -z "$crates" ]; then
    packages=--workspace
    doc_packages=--workspace
    workspace_tests=true
    return
  fi
  packages=""
  doc_packages=""
  workspace_tests=false
  for crate in $crates; do
    packages="$packages -p $crate"
    # `.config/nextest.toml`'s `ci-workspace` profile holds xtask's tests that run cargo on this target, and validation's
    # one; every crate that changes validation reaches xtask too, so xtask alone says whether the step has any.
    if [ "$crate" = xtask ]; then workspace_tests=true; fi
  done
  packages=${packages# }
  for crate in $libraries; do
    doc_packages="$doc_packages -p $crate"
  done
  doc_packages=${doc_packages# }
}

decide() {
  local tier="${TIER:?TIER is not set}" pull_request=false quick=false
  if [ "${GITHUB_EVENT_NAME:-}" = pull_request ]; then pull_request=true; fi
  packages=""
  doc_packages=""
  workspace_tests=false
  if [ "$tier" = code ]; then
    if ! full_tier; then quick=true; fi
  fi
  if [ "$quick" = true ]; then
    quick_packages
  fi
  output quick "$quick"
  output packages "$packages"
  output doc_packages "$doc_packages"
  output workspace_tests "$workspace_tests"
  # `docs_only` is "runs no full CI", as R-677 reads it, and the GPU suites are full-only: `ci.yml`'s full-only jobs skip
  # on the two of them, on a quick run as on a docs one.
  if [ "$quick" = true ]; then
    output docs_only true
    output gpu false
  else
    output docs_only "${DOCS_ONLY:?DOCS_ONLY is not set}"
    output gpu "${GPU:?GPU is not set}"
  fi
  if [ "$tier" = code ] && [ "$pull_request" = true ]; then
    output report "$([ "$quick" = true ] && echo false || echo true)"
    local why="Runs on the merge-ready label (R-684)" check
    if [ "$quick" = false ]; then why="The full tier is running (R-684)"; fi
    for check in $FULL_ONLY; do
      # A failed post leaves the check unreported, which branch protection reads as expected: still no pass.
      post "$check" pending "$why" || echo "::warning::ci_tier: could not post $check pending on $HEAD_SHA"
    done
  else
    output report false
  fi
}

report() {
  local check result state failed=0
  for check in $FULL_ONLY; do
    # RESULT_ci, RESULT_xtask_ci, …: each gate's `needs.<gate>.result`.
    local var="RESULT_${check//-/_}"
    result="${!var:?$var is not set}"
    case "$result" in
      success) state=success ;;
      failure) state=failure ;;
      *) state=pending ;;
    esac
    echo "$check: $result, posted $state"
    post "$check" "$state" "The full tier's gate ended $result (R-684)" || failed=1
  done
  return "$failed"
}

# The latest `pull_request` run of `ci.yml` on the head, as `<id> <status> <attempt>`; empty if there is none yet.
latest_run() {
  gh api "repos/$GH_REPO/actions/workflows/ci.yml/runs?head_sha=$HEAD_SHA&event=pull_request&per_page=100" \
    --jq '.workflow_runs[0] // empty | "\(.id) \(.status) \(.run_attempt)"'
}

# Whether every full-only check passed in run $1's latest attempt: the full tier has passed on this head, or the head is
# a docs or data one, whose gates pass without it. Nothing to re-run then.
already_passed() {
  local passed check
  passed=$(gh api "repos/$GH_REPO/actions/runs/$1/jobs?filter=latest&per_page=100" \
    --jq '.jobs[] | select(.conclusion == "success") | .name')
  for check in $FULL_ONLY; do
    grep -qx "$check" <<< "$passed" || return 1
  done
}

# Waits for run $1 to end. A cancelled run still runs its `always()` gates, which can queue behind the concurrent-job
# cap, so after CI_TIER_POLLS polls it is force-cancelled; if it has still not ended after as many again, this fails.
cancel_and_wait() {
  local id=$1 polls="${CI_TIER_POLLS:-12}" i run status
  gh api --method POST "repos/$GH_REPO/actions/runs/$id/cancel" > /dev/null || true
  for ((i = 1; i <= 2 * polls; i++)); do
    run=$(latest_run)
    status=$(cut -d' ' -f2 <<< "$run")
    if [ "$status" = completed ]; then return 0; fi
    if [ "$i" = "$polls" ]; then
      gh api --method POST "repos/$GH_REPO/actions/runs/$id/force-cancel" > /dev/null || true
    fi
    sleep "${CI_TIER_POLL_SECONDS:-5}"
  done
  echo "::error::ci_tier: run $id did not end after it was cancelled"
  return 1
}

label_on() {
  local polls="${CI_TIER_POLLS:-12}" i run="" id status live
  for ((i = 1; i <= polls; i++)); do
    run=$(latest_run)
    if [ -n "$run" ]; then break; fi
    sleep "${CI_TIER_POLL_SECONDS:-5}"
  done
  if [ -z "$run" ]; then
    echo "::error::ci_tier: no pull_request run of ci.yml on $HEAD_SHA to re-run"
    return 1
  fi
  read -r id status _ <<< "$run"
  if [ "$status" = completed ] && already_passed "$id"; then
    echo "run $id already passed every full-only check on $HEAD_SHA: nothing to re-run"
    return 0
  fi
  if [ "$status" != completed ]; then
    cancel_and_wait "$id"
  fi
  # The head again, now, after any wait: a push has its own run, and re-running the old head's would cancel it.
  live=$(gh api "repos/$GH_REPO/pulls/$PR" --jq .head.sha)
  if [ "$live" != "$HEAD_SHA" ]; then
    echo "$HEAD_SHA is no longer the head ($live): nothing to re-run"
    return 0
  fi
  echo "re-running run $id on $HEAD_SHA: the new attempt finds merge-ready on and runs the full tier"
  gh api --method POST "repos/$GH_REPO/actions/runs/$id/rerun" > /dev/null
}

label_off() {
  local run id status attempt
  run=$(latest_run)
  if [ -z "$run" ]; then return 0; fi
  read -r id status attempt <<< "$run"
  # A first attempt is the head's quick run: it runs whatever the label says.
  if [ "$status" != completed ] && [ "$attempt" != 1 ]; then
    echo "merge-ready is off $HEAD_SHA: cancelling its full attempt in run $id"
    gh api --method POST "repos/$GH_REPO/actions/runs/$id/cancel" > /dev/null
  fi
}

merge_ready() {
  case "${ACTION:-}" in
    labeled) label_on ;;
    unlabeled) label_off ;;
    *)
      echo "ci_tier: merge-ready on the event action '${ACTION:-}'" >&2
      return 2
      ;;
  esac
}

case "${1:-}" in
  decide) decide ;;
  report) report ;;
  merge-ready) merge_ready ;;
  *)
    echo "usage: .github/scripts/ci_tier.sh decide | report | merge-ready" >&2
    exit 2
    ;;
esac
