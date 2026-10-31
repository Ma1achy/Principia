#!/usr/bin/env bash
# Keeps what a mutant does to processes inside the mutants run (R-658; applied per R-369). The job queue's code signals
# process groups and starts daemons, so a mutant of it can do anything a user's process can; on 31 Oct 2026 every shard
# of two runs in a row (41161766829, 41178118082) ended "The hosted runner lost communication", logs gone. The hosted
# runner's agent runs as the same user as the tests, so `kill(-1)` reaches it, and it shares the machine's memory, tasks
# and disk with them. This holds whatever the mutated code does:
#
# A workflow step uses `run` or `check` as its `shell:`, so its script reads as it would otherwise.
#
#   scripts/contain.sh run <command...>   With PRIN_CONTAIN=required (CI's mutants steps): runs the command, as this
#                                         user, in a PID namespace of its own and a cgroup scope with memory, task and
#                                         CPU limits. Signals reach only the namespace, whose init is root's, so
#                                         `kill(-1)` ends the run, not the runner, and the step fails, named; all of it
#                                         dies with the namespace, and a signal to this script (a step's limit, a
#                                         cancel) ends the scope; an out-of-memory kill stays inside the scope. A
#                                         heartbeat outside it reports, and stops the run before the disk fills. If
#                                         these can't be had, it fails: CI never runs this code uncontained. Without
#                                         PRIN_CONTAIN, it just runs the command: a local run, or a test's stand-in.
#   scripts/contain.sh test <command...>  nextest's wrapper of the queue's tests (.config/nextest.toml). With
#                                         PRIN_TEST_PIDNS=1, which `run` sets, runs one test process in a PID namespace
#                                         of its own: whatever it starts is gone when it ends, and nothing it signals
#                                         is outside it, so a mutant's `kill(-1)` fails that test and no other.
#                                         Without it, just runs the command.
#   scripts/contain.sh check [<script>]   With PRIN_CONTAIN=required: proves both on this machine, with a real
#                                         `kill -9 -1` inside each; fails if the containment doesn't hold. Then runs
#                                         the script, if one is given.
#
# The heartbeat survives the runner's death: while a run lasts, every 2 minutes, it sets a commit status (context
# HEARTBEAT_CONTEXT on HEARTBEAT_SHA, with HEARTBEAT_TOKEN) to the memory, disk, copy-on-write store, tasks, out-of-memory
# kills, the number of logs and the newest log under HEARTBEAT_OUT/log, the mutant under test (`baseline.log` while the
# baseline runs). A
# finished run's says "finished". GitHub keeps a status when a job's log is lost, so after a death
# `gh api repos/<repo>/commits/<sha>/statuses` names what was running and how the machine stood. Always state `success`:
# it reports, and never holds a merge. The token is kept from the contained command.
set -euo pipefail

# Under each: an init that stays while the command runs, so that orphans are reaped; a shell's wait takes any child. The
# command gets a session of its own, so its process group is one the namespace can see: a group outside it reads as 0,
# which getpgrp's callers don't expect (rustix panics). Group signals from outside, nextest's, then reach only the init,
# which passes on a SIGTERM. A SIGKILL to the outer group kills `unshare`, and with it the namespace.
# shellcheck disable=SC2016 # expanded by the init
INIT='trap '\''kill -TERM "$child" 2> /dev/null'\'' TERM INT
setsid "$@" &
child=$!
while :; do
  wait "$child"; rc=$?
  kill -0 "$child" 2> /dev/null || exit "$rc"
done'
# shellcheck disable=SC2016 # expanded inside
# The shell stays, as root, so a `kill(-1)` from the command leaves it to note the scope's out-of-memory kills as the scope
# ends: nothing outside can read them afterwards.
RUN_AS='uid=$1 gid=$2 envfile=$3 limits=$4; shift 4
eval "$limits"
setpriv --reuid="$uid" --regid="$gid" --init-groups -- bash -c '\''. "$1"; shift; exec "$@"'\'' contain-env "$envfile" "$@"
rc=$?
awk '\''$1 == "oom_kill" { print $2 }'\'' "/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup 2> /dev/null)/memory.events" \
  > "$envfile.oom" 2> /dev/null
exit "$rc"'

# Memory: the runner has 16 GiB and no swap (mutants.yml removes it); this leaves its agent and the kernel the rest.
MEMORY_MAX=14500M
TASKS_MAX=16384
# Below this much free disk the run is stopped: a full disk takes the agent's own logging with it.
DISK_FLOOR_GIB=3
COW_CEILING_PERCENT=97
SAMPLE_SECS=30
POST_EVERY=4

die() {
  echo "::error::contain.sh: $*" >&2
  exit 125
}

require_tools() {
  [ "$(uname -s)" = Linux ] || die "containment required, and this is not Linux"
  for tool in sudo systemd-run systemctl unshare setpriv; do
    command -v "$tool" > /dev/null || die "containment required, and there is no $tool"
  done
  sudo -n true 2> /dev/null || die "containment required, and sudo needs a password"
  # ubuntu-24.04 refuses unprivileged user namespaces by default, which `test` needs. Absent on other kernels.
  sudo -n sysctl -q -w kernel.apparmor_restrict_unprivileged_userns=0 2> /dev/null || true
}

# One sample of the machine, in a status description's 140 characters.
sample() {
  local unit=$1 began=$2 mem disk tasks group oom logs newest
  mem=$(awk '/^MemAvailable:/ { printf "%.1f", $2 / 1048576 }' /proc/meminfo)
  disk=$(df -BG --output=avail / | tail -1 | tr -dc 0-9)
  tasks=$(cut -d' ' -f4 /proc/loadavg)
  group=$(systemctl show -p ControlGroup --value "$unit.scope" 2> /dev/null || true)
  oom=${final_oom:-$(awk '$1 == "oom_kill" { print $2 }' "/sys/fs/cgroup$group/memory.events" 2> /dev/null || true)}
  logs=$(find "${HEARTBEAT_OUT:-/nonexistent}/log" -type f 2> /dev/null | wc -l || true)
  # The newest log's name, its last 48 characters: a mutant's file, line and column.
  newest=$(ls -t "${HEARTBEAT_OUT:-/nonexistent}/log" 2> /dev/null | head -1 | tail -c 48 || true)
  printf '+%dm mem %sG disk %sG cow %s tasks %s oom %s logs %s %s' "$((($(date +%s) - began) / 60))" \
    "$mem" "$disk" "$(cow_percent)" "$tasks" "${oom:-?}" "$logs" "$newest"
}

# How full the root's copy-on-write store is, where it has one. Some hosted runners have one (`/dev/mapper/cow-root`,
# 72G, against the usual 145G `/dev/root`: mutants shard 3 of run 41193733373). Deleting files doesn't empty such a
# store, so `df` may show room it doesn't have, and a store that fills fails the machine's whole root disk. `-` where
# there is none.
cow_percent() {
  sudo -n dmsetup status 2> /dev/null | awk '$4 == "snapshot" { split($5, u, "/"); if (u[2] > 0) { printf "%d", 100 * u[1] / u[2]; found = 1 } }
    END { if (!found) printf "-" }' || true
}

post() {
  [ -n "${HEARTBEAT_CONTEXT:-}" ] && [ -n "${token:-}" ] && [ -n "${HEARTBEAT_SHA:-}" ] || return 0
  curl -fsS -m 20 -o /dev/null -X POST -H "Authorization: Bearer $token" -H "Accept: application/vnd.github+json" \
    "${GITHUB_API_URL:-https://api.github.com}/repos/${GITHUB_REPOSITORY:-}/statuses/$HEARTBEAT_SHA" \
    -d "{\"state\":\"success\",\"context\":\"$HEARTBEAT_CONTEXT\",\"description\":\"${1:0:140}\"}" 2> /dev/null || true
}

heartbeat() {
  local unit=$1 began=$2 n=0 line free cow
  # A failed sample is not the end of the heartbeat.
  set +e
  while :; do
    line=$(sample "$unit" "$began")
    echo "$line" >> "${RUNNER_TEMP:-/tmp}/heartbeat.log"
    if [ $((n % POST_EVERY)) -eq 0 ]; then
      echo "heartbeat: $line"
      post "$line"
    fi
    n=$((n + 1))
    free=$(df -BG --output=avail / | tail -1 | tr -dc 0-9)
    cow=$(cow_percent)
    if [ "${free:-0}" -lt "$DISK_FLOOR_GIB" ] || { [ "$cow" != - ] && [ "${cow:-0}" -ge "$COW_CEILING_PERCENT" ]; }; then
      echo "::error::contain.sh: ${free} GiB of disk left, copy-on-write store ${cow}% full: stopping the run before it takes the runner"
      post "disk floor: stopped. $line"
      sudo -n systemctl kill --signal=SIGKILL "$unit.scope" || true
    fi
    sleep "$SAMPLE_SECS" > /dev/null 2>&1
  done
}

# A signal to this script: the scope runs as root, out of this user's reach, and would go on through the report's upload.
stop_run() {
  stopped=$((128 + $1))
  echo "::error::contain.sh: stopped by signal $1: ending the contained run"
  sudo -n systemctl kill --signal=SIGKILL "$unit.scope" 2> /dev/null || true
  kill "${watcher:-}" 2> /dev/null || true
}

# The scope's last out-of-memory count: the one noted as it ended, or, if it was killed whole, the run's last sample's.
last_oom() {
  local noted
  noted=$(cat "$envfile.oom" 2> /dev/null || true)
  if [ -n "$noted" ]; then
    echo "$noted"
  else
    awk -v run="--- $unit" '$0 == run { oom = "" } match($0, / oom [0-9]+ /) { oom = substr($0, RSTART + 5, RLENGTH - 6) }
      END { print oom }' "${RUNNER_TEMP:-/tmp}/heartbeat.log" 2> /dev/null || true
  fi
}

run_contained() {
  [ "${PRIN_CONTAIN:-}" = required ] || exec "$@"
  require_tools
  local began uid gid rc limits limit final_oom=""
  unit="prin-contain-$$"
  stopped=""
  began=$(date +%s)
  uid=$(id -u)
  gid=$(id -g)
  token=${HEARTBEAT_TOKEN:-}
  unset HEARTBEAT_TOKEN PRIN_CONTAIN
  export PRIN_TEST_PIDNS=1
  envfile=$(mktemp "${RUNNER_TEMP:-/tmp}/contain-env.XXXXXX")
  # sudo strips the environment; the command gets all of this one, PATH and LD_LIBRARY_PATH included.
  declare -px > "$envfile"
  # sudo takes the core-dump limit to 0, and only root may raise a hard limit: `prlimit --core=1`, the mutants' memory
  # cap's runner, then fails on every test. The command gets this shell's limits back, set as root.
  limits=""
  for limit in c n s u l v; do
    limits+="ulimit -H$limit $(ulimit -H$limit) 2> /dev/null; ulimit -S$limit $(ulimit -S$limit) 2> /dev/null; "
  done
  df -h / || true
  sudo -n dmsetup status 2> /dev/null || true
  echo "--- $unit" >> "${RUNNER_TEMP:-/tmp}/heartbeat.log"
  heartbeat "$unit" "$began" &
  watcher=$!
  trap 'stop_run 15' TERM
  trap 'stop_run 2' INT
  trap 'stop_run 1' HUP
  set +e
  sudo -n systemd-run --scope --quiet --unit="$unit" -p MemoryMax="$MEMORY_MAX" -p TasksMax="$TASKS_MAX" \
    -p CPUWeight=20 -p IOWeight=20 -p OOMPolicy=continue -- \
    unshare --pid --fork --kill-child --mount-proc \
    bash -c "$INIT" contain-init bash -c "$RUN_AS" contain-user "$uid" "$gid" "$envfile" "$limits" "$@" \
    < /dev/null &
  # In the background, so that a signal is acted on now, not when the foreground command ends.
  wait "$!"
  rc=$?
  set -e
  trap - TERM INT HUP
  rc=${stopped:-$rc}
  kill "$watcher" 2> /dev/null || true
  final_oom=$(last_oom)
  rm -f "$envfile" "$envfile.oom"
  post "finished, exit $rc. $(sample "$unit" "$began")"
  if [ "$rc" -eq 137 ]; then
    echo "::error::contain.sh: the contained run was killed (SIGKILL): a kill(-1), the out-of-memory killer" \
      "(oom ${final_oom:-?}) or the disk floor; the heartbeat's last samples follow. Its outcomes so far are in the report."
  fi
  if [ "$rc" -ne 0 ]; then
    echo "contain.sh: exit $rc, oom ${final_oom:-?}; the heartbeat's last samples:"
    tail -n 40 "${RUNNER_TEMP:-/tmp}/heartbeat.log" 2> /dev/null || true
  fi
  return "$rc"
}

run_test() {
  [ "${PRIN_TEST_PIDNS:-}" = 1 ] || exec "$@"
  exec unshare --user --map-current-user --pid --fork --kill-child --mount-proc bash -c "$INIT" contain-init "$@"
}

# Each containment: `kill -9 -1` inside must leave a process of this user outside alive, must not see it, and what the
# command leaves behind must end with it. The control: the same look from outside must see that process.
check() {
  if [ "${PRIN_CONTAIN:-}" != required ]; then
    echo "contain.sh check: containment not required here"
    return 0
  fi
  require_tools
  sleep 600 > /dev/null 2>&1 &
  local outer=$! self=$0 mark=299.7531 leak
  kill -0 "$outer" || die "check: the control can't see its own process $outer"
  # shellcheck disable=SC2016 # expanded inside
  local probe='[ "$(cat /proc/1/comm)" = bash ] || exit 41
    if kill -0 "$1" 2> /dev/null; then exit 42; fi
    [ "$(ps -o pgid= -p $$)" -gt 0 ] || exit 43
    prlimit --as=8589934592 --core=1 true || exit 44
    kill -9 -1 2> /dev/null
    setsid sleep "$2" < /dev/null > /dev/null 2>&1 &
    exit 0'
  local mode
  for mode in run "run $self test"; do
    # shellcheck disable=SC2086 # the mode's words
    ( "$self" $mode bash -c "$probe" probe "$outer" "$mark" ) || die "check: inside \`$mode\`: exit $?"
    kill -0 "$outer" 2> /dev/null || die "check: \`kill -9 -1\` inside \`$mode\` reached a process outside"
    leak=$(pgrep -fc "sleep $mark" || true)
    [ "$leak" -eq 0 ] || die "check: $leak process(es) outlived \`$mode\`"
    echo "contain.sh check: \`$mode\` holds: kill -9 -1 stayed inside, outside unseen, nothing left"
  done
  kill "$outer"
}

mode=${1:-}
[ "$#" -gt 0 ] && shift
case "$mode" in
  run) run_contained "$@" ;;
  test) run_test "$@" ;;
  check)
    check
    [ "$#" -eq 0 ] || exec bash "$@"
    ;;
  *) die "usage: contain.sh run|test <command...> | check [<script>]" ;;
esac
