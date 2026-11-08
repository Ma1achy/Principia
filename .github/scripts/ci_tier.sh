#!/usr/bin/env bash
# CI's two code tiers (R-684): full CI on the merge-candidate head, the one carrying the `merge-ready` label, and on
# every push to `main`; the quick tier on every other pull-request head. R-677's classifier applies first: a docs or
# data head runs neither.
#
#   .github/scripts/ci_tier.sh decide        In `ci.yml`'s `changes` job, after `scripts/ci_changes.sh detect`: this run's
#                                            tier, written to $GITHUB_OUTPUT, and on a full attempt what an earlier
#                                            attempt's quick tier passed. On a code head each full-only required check
#                                            is posted pending.
#   .github/scripts/ci_tier.sh report        In `ci.yml`'s `ci-status` job: each full-only check's status, from its gate.
#   .github/scripts/ci_tier.sh merge-ready   In `ci-full.yml`: `merge-ready` put on starts the head's full tier, without
#                                            a push, at once, cancelling a quick attempt still going; taken off, it
#                                            cancels a full attempt still going.
#
# How a head goes from quick to full. A label change must not start a run of `ci.yml`: of each workflow's runs on a head
# branch protection counts only the latest run's check suite (R-678), so another label's run would supersede the suite
# holding the head's required checks. So `ci.yml` runs on no label change. `ci-full.yml` re-runs the head's `pull_request`
# run instead, as `reviews.yml` re-runs its own (R-276): a new attempt of the same run, in the same check suite, on the
# same merge commit. The new attempt finds the label on and is full.
#
# The full tier adds to the quick tier (the owner, 7 Nov 2026: "The full tier should compose / add on to the quick
# tier"), without waiting for it (the owner, 8 Nov 2026: "ci-full: on merge-ready, cancel the in-progress quick run and
# start the full tier, instead of waiting for it"). A quick attempt still going when the label goes on is cancelled and
# the full attempt started at once. The full attempt skips what the quick tier passed on that merge commit: `decide`'s
# `quick_passed`, an earlier attempt of this run whose `ci-quick` job passed, on which `ci-checks` skips fmt, clippy and
# the build, the same commands; and `quick_covers_ci`, that attempt having tested every crate, on which `ci-archive` and
# the `ci` shards skip, and `ci-workspace` skips the tests and doctests quick ran, the same tests under the same nextest
# profiles. Only what can be shown counts: an attempt whose jobs cannot be read is no pass, and changed paths that
# cannot be read, or none, cover no crate. A quick tier of some crates covers no test: the shards then run whole. A
# cancelled quick attempt passed nothing it had not finished: its `ci-quick` job, unless it had already passed,
# concluded `cancelled`, and the full attempt runs it all again.
#
# Pending, never skipped-as-success. On a quick run no job bears a required name, so a skip cannot pass one: a gate
# skipped at job level reports `matrix.check` (run 41972273540; `ci.yml`, the gates' comment), and until the full tier
# runs branch protection reads the check as expected. `decide` also posts each one pending as a commit status, which belongs to no
# check suite and so no later run supersedes. When a check and a commit status share a name, GitHub requires both, so
# `report` always closes what `decide` opened; that holds whether or not branch protection's app binding
# (`required_status_checks`, app 15368) counts a status at all.
set -euo pipefail

# The required checks only the full tier reports. `pr-check`, `reviews-complete` and `mutants-check` report on every
# head (R-678).
FULL_ONLY="ci xtask-ci gpu-kernel gpu-metal gpu-lavapipe"

# The quick tier's job as the jobs API names it: `ci.yml`'s job `ci-quick` calling `ci-quick.yml`'s job `ci-quick`.
QUICK_JOB="ci-quick / ci-quick"

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

# What an earlier attempt of this run passed of the quick tier, on this attempt's merge commit (a re-run keeps its
# commit): sets `quick_passed`, some earlier attempt's `ci-quick` job concluded `success`, and `quick_covers_ci`, that
# job tested every crate, its tests and doctests then those of the `ci` shards and `ci-workspace`. Only what is shown
# counts: an attempt whose jobs cannot be read is no pass, and changed paths that cannot be read, or none, cover no
# crate, where `quick_packages` fails towards every test, so never more than the quick tier ran.
quick_evidence() {
  local attempt="${GITHUB_RUN_ATTEMPT:-1}" n names paths crates every
  local changes="${CI_CHANGES:-$here/../../scripts/ci_changes.sh}" script="$here/ci_quick_crates.py"
  local workspace="${CI_ROOT:-.}"
  quick_passed=false
  quick_covers_ci=false
  for ((n = 1; n < attempt; n++)); do
    names=$(gh api "repos/$GH_REPO/actions/runs/${GITHUB_RUN_ID:?}/attempts/$n/jobs?per_page=100" \
      --jq '.jobs[] | select(.conclusion == "success") | .name') || continue
    if grep -qxF "$QUICK_JOB" <<< "$names"; then
      quick_passed=true
      echo "attempt $n of this run passed the quick tier on this merge commit"
      break
    fi
  done
  if [ "$quick_passed" = false ]; then return 0; fi
  paths=$("$changes" paths) || return 0
  if [ -z "$paths" ]; then return 0; fi
  crates=$(python3 -I "$script" --root "$workspace" <<< "$paths") || return 0
  every=$(python3 -I "$script" --root "$workspace" < /dev/null) || return 0
  if [ -n "$crates" ] && [ "$crates" = "$every" ]; then
    quick_covers_ci=true
    echo "it tested every crate: the ci shards' and ci-workspace's tests"
  fi
}

decide() {
  local tier="${TIER:?TIER is not set}" pull_request=false quick=false
  if [ "${GITHUB_EVENT_NAME:-}" = pull_request ]; then pull_request=true; fi
  packages=""
  doc_packages=""
  workspace_tests=false
  quick_passed=false
  quick_covers_ci=false
  if [ "$tier" = code ]; then
    if ! full_tier; then quick=true; fi
  fi
  if [ "$quick" = true ]; then
    quick_packages
  elif [ "$tier" = code ] && [ "$pull_request" = true ]; then
    quick_evidence
  fi
  output quick "$quick"
  output quick_passed "$quick_passed"
  output quick_covers_ci "$quick_covers_ci"
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

# Waits for the head's latest run to end: `wait_for_end <polls> <seconds>` polls every <seconds>, at most <polls> times;
# prints the ended run, or fails, named, when it has not ended.
wait_for_end() {
  local polls=$1 i run status
  for ((i = 1; i <= polls; i++)); do
    sleep "$2"
    run=$(latest_run)
    status=$(cut -d' ' -f2 <<< "$run")
    if [ "$status" = completed ]; then
      echo "$run"
      return 0
    fi
  done
  echo "::error::ci_tier: the run on $HEAD_SHA had not ended after $polls polls; the full tier did not start" >&2
  return 1
}

# A full attempt still going is the full tier already, and is waited for, never cancelled: every CI_TIER_WAIT_SECONDS
# (30, or CI_TIER_POLL_SECONDS where set), at most CI_TIER_WAIT_POLLS times (700: 350 minutes, under the job's 360), less
# the polls `attempt_tier` has already spent waiting for the attempt's `changes` to decide (`waited`).
wait_full() {
  wait_for_end "$((${CI_TIER_WAIT_POLLS:-700} - waited))" "${CI_TIER_WAIT_SECONDS:-${CI_TIER_POLL_SECONDS:-30}}"
}

# What attempt $2 of run $1 shows of its tier, from one read of its jobs: 0, shown full, its `changes` job having ended
# and `ci-quick` skipped, under either name the jobs API gives it; 1, shown quick, `changes` having ended and `ci-quick`
# not skipped; 2, not yet decided, `changes` queued or still going; 3, its jobs unread.
shown_tier() {
  local jobs
  jobs=$(gh api "repos/$GH_REPO/actions/runs/$1/attempts/$2/jobs?per_page=100" \
    --jq '.jobs[] | "\(.conclusion) \(.name)"') || return 3
  if ! grep -qxE '[a-z_]+ changes' <<< "$jobs" || grep -qxF 'null changes' <<< "$jobs"; then return 2; fi
  if grep -qxF -e 'skipped ci-quick' -e "skipped $QUICK_JOB" <<< "$jobs"; then return 0; fi
  return 1
}

# Whether attempt $2 of run $1 is the full tier: 0, shown full, waited for and never cancelled; 1, shown quick, which
# alone is cancelled, losing nothing, the full attempt running all a cancelled one had not passed. Its `changes` not yet
# ended (queued, perhaps long, behind a busy runner queue), it is read again until `changes` decides, reading the live
# label, every CI_TIER_WAIT_SECONDS, within the same CI_TIER_WAIT_POLLS as `wait_full`, each such poll counted in
# `waited`; its jobs unread, it is read again every CI_TIER_DECIDE_SECONDS (10, or CI_TIER_POLL_SECONDS where set), at
# most CI_TIER_DECIDE_POLLS reads (30: 5 minutes). Either still not shown, it fails 2, named, cancelling nothing, as
# `wanted` does on an unread head: an attempt whose tier is not shown, a retry's full attempt just started among them,
# is never taken for a quick one.
attempt_tier() {
  local wait_polls="${CI_TIER_WAIT_POLLS:-700}" wait_seconds="${CI_TIER_WAIT_SECONDS:-${CI_TIER_POLL_SECONDS:-30}}"
  local read_polls="${CI_TIER_DECIDE_POLLS:-30}" read_seconds="${CI_TIER_DECIDE_SECONDS:-${CI_TIER_POLL_SECONDS:-10}}"
  local unread=0 shown what="attempt $2 of run $1 on $HEAD_SHA"
  while true; do
    shown=0
    shown_tier "$1" "$2" || shown=$?
    case "$shown" in
      0 | 1) return "$shown" ;;
      2)
        waited=$((waited + 1))
        if [ "$waited" -ge "$wait_polls" ]; then
          echo "::error::ci_tier: $what had not decided its tier after $waited polls; cancelling nothing"
          return 2
        fi
        sleep "$wait_seconds"
        ;;
      *)
        unread=$((unread + 1))
        if [ "$unread" -ge "$read_polls" ]; then
          echo "::error::ci_tier: the jobs of $what could not be read in $unread reads; cancelling nothing"
          return 2
        fi
        sleep "$read_seconds"
        ;;
    esac
  done
}

# Cancels run $1's quick attempt, still going, so the full attempt starts at once (the owner, 8 Nov 2026: "ci-full: on
# merge-ready, cancel the in-progress quick run and start the full tier, instead of waiting for it"), and waits for the
# cancellation to end it: every CI_TIER_CANCEL_SECONDS (10, or CI_TIER_POLL_SECONDS where set), at most
# CI_TIER_CANCEL_POLLS times (30: 5 minutes). A run the cancel has not ended by then is force-cancelled, once, and waited
# for as long again. `latest_run` reads `pull_request` runs alone, so this never cancels a run on `main` (R-656). A
# refused cancel is waited out too: the run may have ended meanwhile. Prints the ended run, or fails, named.
cancel_quick() {
  local polls="${CI_TIER_CANCEL_POLLS:-30}" seconds="${CI_TIER_CANCEL_SECONDS:-${CI_TIER_POLL_SECONDS:-10}}"
  gh api --method POST "repos/$GH_REPO/actions/runs/$1/cancel" > /dev/null \
    || echo "::warning::ci_tier: the cancel of run $1 was refused" >&2
  if wait_for_end "$polls" "$seconds"; then return 0; fi
  echo "run $1 had not ended $polls polls after its cancel: force-cancelling it" >&2
  gh api --method POST "repos/$GH_REPO/actions/runs/$1/force-cancel" > /dev/null \
    || echo "::warning::ci_tier: the force-cancel of run $1 was refused" >&2
  wait_for_end "$polls" "$seconds"
}

# Whether the head still wants its full tier: $HEAD_SHA still the head, now (a push has its own run, and re-running or
# cancelling the old head's would cancel it, or cost it its quick tier for nothing), and `merge-ready` still on (a
# push's `merge-ready-off` takes it off with a workflow's token, which starts no run of this one). Fails 1, saying why,
# when it does not; 2 when the head or the labels cannot be read.
wanted() {
  local live labels
  live=$(gh api "repos/$GH_REPO/pulls/$PR" --jq .head.sha) || {
    echo "::error::ci_tier: could not read pull request $PR's head"
    return 2
  }
  if [ "$live" != "$HEAD_SHA" ]; then
    echo "$HEAD_SHA is no longer the head ($live): nothing to re-run"
    return 1
  fi
  labels=$(live_labels) || {
    echo "::error::ci_tier: could not read pull request $PR's labels"
    return 2
  }
  if ! grep -qxF merge-ready <<< "$labels"; then
    echo "merge-ready is no longer on pull request $PR: nothing to re-run"
    return 1
  fi
}

label_on() {
  local polls="${CI_TIER_POLLS:-12}" i run="" id status attempt round why shown waited=0
  for ((i = 1; i <= polls; i++)); do
    run=$(latest_run)
    if [ -n "$run" ]; then break; fi
    sleep "${CI_TIER_POLL_SECONDS:-5}"
  done
  if [ -z "$run" ]; then
    echo "::error::ci_tier: no pull_request run of ci.yml on $HEAD_SHA to re-run"
    return 1
  fi
  read -r id status attempt <<< "$run"
  # A second round only when the re-run is refused because the run started again meanwhile (`retry.yml`'s re-run of a
  # runner fault): that attempt is cancelled too, if shown quick, or waited for, if full; not shown either, it fails.
  for round in 1 2; do
    if [ "$status" != completed ]; then
      shown=0
      attempt_tier "$id" "$attempt" || shown=$?
      if [ "$shown" = 0 ]; then
        echo "run $id on $HEAD_SHA is a full attempt, $status: waiting for it to end"
        run=$(wait_full) || return 1
      elif [ "$shown" != 1 ]; then
        return 1
      else
        why=0
        wanted || why=$?
        if [ "$why" = 1 ]; then return 0; fi
        if [ "$why" != 0 ]; then return 1; fi
        echo "run $id on $HEAD_SHA is a quick attempt, $status: cancelling it to start the full tier now"
        run=$(cancel_quick "$id") || return 1
      fi
      read -r id status attempt <<< "$run"
    fi
    if already_passed "$id"; then
      echo "run $id already passed every full-only check on $HEAD_SHA: nothing to re-run"
      return 0
    fi
    # The head and the label again, now, after any wait.
    why=0
    wanted || why=$?
    if [ "$why" = 1 ]; then return 0; fi
    if [ "$why" != 0 ]; then return 1; fi
    echo "re-running run $id on $HEAD_SHA: the new attempt finds merge-ready on and runs the full tier"
    if gh api --method POST "repos/$GH_REPO/actions/runs/$id/rerun" > /dev/null; then
      return 0
    fi
    run=$(latest_run)
    read -r id status attempt <<< "$run"
    if [ "$status" = completed ] || [ "$round" = 2 ]; then
      echo "::error::ci_tier: run $id on $HEAD_SHA could not be re-run"
      return 1
    fi
  done
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
