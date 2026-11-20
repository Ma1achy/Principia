#!/usr/bin/env bash
# What `.github/scripts/wsl.ps1` runs inside the WSL2 distro `principia-wsl`, as `ci`, from the clone at the job's
# commit (/home/ci/work/principia). Windows can't look inside, and inside can't reach Windows (interop is off), so each
# verb prints its evidence and exits 0 only if what it proves held; the Windows side reads the status and the output.
# The parity verb also runs on GitHub's hosted Linux runner, unchanged, for proof 6's other side.
# The addresses address, gateway and listen print are wsl.ps1's to read, raw, for proof 3's decision; wsl.ps1 masks the
# tailnet's in everything it prints (Hide-Tailnet), so nothing here masks them.
#
#   inside.sh identity               who runs here: `whoami`, `id`, the kernel and the release (proof 1)
#   inside.sh isolation              no drive and no Windows program reachable from here (proof 2)
#   inside.sh address                the distro's own IPv4 address
#   inside.sh gateway [<route file>] the default route's gateway, from /proc/net/route or <route file>: in WSL's NAT
#                                    mode, the Windows host on WSL's virtual switch (proof 3, outbound)
#   inside.sh listen <port> <secs>   a TCP listener on every address, for <secs>; prints `listening`, then `accepted <ip>`
#                                    for each connection (proof 3, inbound)
#   inside.sh connect <host> <port>  one TCP connection, 5 s at most; exits 0 only if it connected (proof 3, outbound)
#   inside.sh check                  `scripts/contain.sh check` in its unprivileged mode, the real `kill -9 -1` (proof 4)
#   inside.sh parity <commit>        proof 6's command at <commit>; its per-test results as `parity.py summarize` prints
#   inside.sh compare                this side's parity results against the other side's, read on stdin (proof 6)
#   inside.sh cpu                    the CPU tests, then the job queue's mutants, each through `scripts/contain.sh run`
#                                    in its unprivileged mode, into ~/results. `wsl.ps1` doesn't run it: the PC runs
#                                    the GPU suites only, and the job queue's mutants stay on GitHub's runners (R-1014,
#                                    R-912), the answer to RQ-825 (closed by R-1029) moving mutation runs alone, measured first
#   inside.sh mutants                one named mutation workload, read on stdin as five lines: the commit, the diff's
#                                    base (an ancestor of it), the packages (space-separated, each a crate under
#                                    crates/ but gui and prin), cargo mutants' --timeout in seconds, and the nextest
#                                    filterset; then two more, each optional, empty or absent for its default: the
#                                    jobs, 1 to 4 (1), and the shard, k/n with k from 0 to n-1 and n from 1 to 99
#                                    (none). Refused, before anything runs, unless each is well formed, and, where
#                                    the jobs are more than 1, unless the distro's memory is at least 8 GiB a job (an
#                                    allowance; an out-of-memory kill during the run fails it, `contain.sh`). Then, through `scripts/contain.sh run` in its unprivileged mode, at
#                                    the commit, `cargo mutants --in-diff <base..commit> --package <each> -j <jobs>
#                                    [--shard <k/n>] --timeout <s> -- -E <filterset>` under nextest's `mutants`
#                                    profile, into ~/results: mutants.out/, the diff, the log, wall.txt (the whole
#                                    run's seconds) and host.txt (the CPU model, the cores, memory, load at start and
#                                    end, the jobs, the shard, and the build cache's state and size). At -j 1 it builds
#                                    in the warm build cache, ~/target, kept between dispatches; at more, each copy
#                                    cargo mutants makes builds in a target of its own (below). The owner's answers on
#                                    RQ-825 (closed by R-1029), 16 and 17 Nov 2026: "yes move mutation runs onto the PC (if they are
#                                    faster there, measure first)", then "i'd move them" and "we could try optimise
#                                    them on the windows machine so its faster?" (R-1016, R-1017); `wsl-cpu` runs
#                                    this, and nothing else, once `trust` has passed
#   inside.sh trust                  the same lines on stdin, of which it reads the commit and the base: refused
#                                    unless the commit is on one of origin's branches in the clone (refs/heads, never
#                                    refs/pull: nothing is fetched by id), the base is on origin/main, and every commit
#                                    from the base to the commit not on origin/main carries a signature that verifies
#                                    against the owner's key pinned in `.github/allowed_signers`, read from the clone at
#                                    the job's commit (R-711 item 6: the signed-commit gate is the primary guard, the
#                                    actor check secondary). `wsl.ps1 cpu` runs it first and stops if it fails
#   inside.sh results                ~/results as a tar stream on stdout
set -euo pipefail
# One stream for Windows to read, but for the tar stream, which is stdout alone, and parity's results, which the hosted
# job writes from stdout to the file `parity.py compare` reads: their stderr stays apart (wsl.ps1 shows it after).
case "${1:-}" in
  results | parity) ;;
  *) exec 2>&1 ;;
esac
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
work="$HOME/work"
# The build settings every workflow's top-level env sets (`win-gpu.yml`), which cross no `wsl.exe` boundary; and the GPU
# backend, Vulkan, which `ci.yml`'s test jobs set too: in the distro it finds lavapipe, as on the hosted runner.
export CARGO_TERM_COLOR=never CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only \
  CARGO_PROFILE_TEST_DEBUG=line-tables-only PRIN_GPU_BACKEND=vulkan
export PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

failed=0
ok() { echo "ok: $*"; }
bad() {
  echo "FAIL: $*"
  failed=1
}
# `holds <what> <command...>`: the command must succeed. `refused <what> <command...>`: it must fail.
holds() {
  local what=$1
  shift
  if "$@"; then ok "$what"; else bad "$what: it doesn't hold"; fi
}
refused() {
  local what=$1
  shift
  if "$@" > /dev/null 2>&1; then bad "$what: it succeeded"; else ok "$what"; fi
}
no_file() { [ ! -e "$1" ]; }
on_path() { command -v "$1"; }

isolation() {
  local drive dir mounts
  for drive in c d C D; do
    holds "/mnt/$drive absent" no_file "/mnt/$drive"
  done
  # A mount backed by Windows: drvfs, or the 9p and virtiofs WSL serves Windows folders over (/usr/lib/wsl/drivers,
  # with GPU support, is the driver store on C:).
  mounts=$(awk '{ for (i = 7; $i != "-"; i++); if ($(i + 1) ~ /^(drvfs|9p|virtiofs)$/ || $(i + 2) ~ /^[A-Za-z]:/) print $5, $(i + 1), $(i + 2) }' \
    /proc/self/mountinfo)
  if [ -z "$mounts" ]; then ok "no Windows-backed mount"; else bad "Windows-backed mounts: ${mounts//$'\n'/; }"; fi
  for dir in /proc/sys/fs/binfmt_misc/WSLInterop /proc/sys/fs/binfmt_misc/WSLInterop-late; do
    holds "$dir absent: no Windows program can start" no_file "$dir"
  done
  for program in cmd.exe powershell.exe pwsh.exe wsl.exe explorer.exe; do
    refused "$program not on PATH" on_path "$program"
  done
  case ":$PATH:" in
    *:/mnt/*) bad "PATH holds a Windows folder: $PATH" ;;
    *) ok "PATH holds no Windows folder" ;;
  esac
  # Mounting a drive again: as ci, and as root of a user namespace of ci's, each type WSL could serve it by.
  dir=$(mktemp -d)
  for type in drvfs 9p virtiofs; do
    refused "ci can't mount C: as $type" mount -t "$type" C: "$dir"
    refused "a user namespace can't mount C: as $type" unshare --user --map-root-user --mount mount -t "$type" C: "$dir"
  done
  rmdir "$dir"
  refused "no sudo" on_path sudo
  holds "no /etc/sudoers" no_file /etc/sudoers
  holds "no /etc/sudoers.d" no_file /etc/sudoers.d
  holds "/etc/wsl.conf is scripts/wsl/wsl.conf, byte for byte" cmp -s /etc/wsl.conf "$repo/scripts/wsl/wsl.conf"
  echo "/mnt holds: $(find /mnt -mindepth 1 -maxdepth 1 -printf '%f ' 2> /dev/null)"
  return "$failed"
}

listen() {
  python3 - "$1" "$2" << 'EOF'
import select, socket, sys, time
s = socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("0.0.0.0", int(sys.argv[1])))
s.listen(8)
print("listening", flush=True)
end = time.time() + float(sys.argv[2])
while (left := end - time.time()) > 0:
    if select.select([s], [], [], left)[0]:
        c, a = s.accept()
        print("accepted", a[0], flush=True)
        c.close()
EOF
}

# The first default route (destination 0.0.0.0) whose RTF_GATEWAY flag (2) is set: its gateway, which the kernel
# writes as a little-endian hex word. Exits 1, naming why, if there is none.
gateway() {
  python3 - "${1:-/proc/net/route}" << 'EOF'
import socket, struct, sys
with open(sys.argv[1]) as f:
    rows = [line.split() for line in f.readlines()[1:]]
for row in rows:
    if len(row) > 3 and row[1] == "00000000" and int(row[3], 16) & 2:
        print(socket.inet_ntoa(struct.pack("<L", int(row[2], 16))))
        sys.exit(0)
print("gateway: no default route through a gateway in", sys.argv[1])
sys.exit(1)
EOF
}

connect() {
  python3 - "$1" "$2" << 'EOF'
import socket, sys
try:
    socket.create_connection((sys.argv[1], int(sys.argv[2])), 5).close()
except OSError as e:
    print("not connected:", e)
    sys.exit(1)
print("connected")
EOF
}

# Proof 6's command, the same on both sides: kernel's tests, deterministic, their GPU harness tests on the Vulkan
# selected above (lavapipe on both sides), under the `ci` profile CI's shards use, no retries, every test run. When
# cargo exits non-zero, the end of its log (cargo's stderr, kept apart from the results) is printed on stderr; and when
# no test ran, the verb fails, after printing it, rather than hand an empty side to `compare` (run 43603140749: cargo
# exited 101 on an invalid config, 0 tests ran, and the verb exited 0).
parity() {
  local commit=$1 dir="$work/parity" start code tests
  rm -rf "$dir"
  git -C "$repo" worktree prune
  # A shallow checkout, as GitHub's runner makes, lacks the commit: fetch it alone.
  git -C "$repo" cat-file -e "$commit^{commit}" 2> /dev/null || git -C "$repo" fetch --quiet --depth 1 origin "$commit"
  git -C "$repo" worktree add --quiet --detach "$dir" "$commit"
  [ "$(git -C "$dir" rev-parse HEAD)" = "$commit" ] || {
    echo "parity: $dir is not at $commit" >&2
    return 1
  }
  start=$(date +%s)
  set +e
  (cd "$dir" && NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run -p kernel --profile ci --no-fail-fast \
    --retries 0 --message-format libtest-json-plus > "$work/parity.json" 2> "$work/parity.log")
  code=$?
  set -e
  python3 "$repo/scripts/wsl/parity.py" summarize "$(($(date +%s) - start))" "$code" < "$work/parity.json" |
    tee "$work/parity.tsv"
  tests=$(sed -n '1s/^# wall .* exit .* tests \([0-9][0-9]*\)$/\1/p' "$work/parity.tsv")
  if [ "$code" -ne 0 ] || [ "${tests:-0}" -eq 0 ]; then
    echo "parity: cargo nextest exited $code, ${tests:-no} tests ran; the end of $work/parity.log:" >&2
    tail -n 40 "$work/parity.log" >&2
  fi
  if [ "${tests:-0}" -eq 0 ]; then
    echo "parity: FAIL: no test ran, so there is nothing to compare" >&2
    return 1
  fi
}

cpu() {
  local results="$HOME/results" code=0 tests mutants
  rm -rf "$results"
  mkdir -p "$results"
  cd "$repo"
  # Not `mutants`' build cache, ~/target: that cache is no other verb's.
  export CARGO_TARGET_DIR="$HOME/target-cpu" PRIN_CONTAIN=unprivileged
  bash scripts/contain.sh check 2>&1 | tee "$results/contain-check.log"
  set +e
  NEXTEST_PROFILE=ci bash scripts/contain.sh run cargo nextest run --workspace --no-fail-fast --retries 0 2>&1 |
    tee "$results/tests.log"
  tests=${PIPESTATUS[0]}
  # The job queue's mutants, as `mutants.yml`'s shards run them: its RUSTFLAGS, the per-test kill limits.
  RUSTFLAGS="-C link-arg=-Wl,--compress-debug-sections=zlib -Zmir-enable-passes=-GVN" NEXTEST_PROFILE=mutants \
    bash scripts/contain.sh run cargo mutants --in-place --package xtask --file xtask/src/jobq.rs -o "$results" 2>&1 |
    tee "$results/mutants.log"
  mutants=${PIPESTATUS[0]}
  set -e
  echo "wsl-cpu: tests exit $tests, mutants exit $mutants"
  [ "$tests" -eq 0 ] || code=$tests
  [ "$mutants" -eq 0 ] || [ "$code" -ne 0 ] || code=$mutants
  return "$code"
}

# A workload input that isn't well formed: named, and nothing run.
refuse() {
  echo "mutants: refused: $*"
  exit 2
}

# Whether the commit id $1 is reachable from one of origin's branches in the clone, `refs/remotes/origin/*`, which
# hold only origin's `refs/heads/*` (`git clone`'s refspec, `wsl.ps1`'s Initialize-Clone). Read from the branches' own
# history, so the id is never looked up as an object, which in the clone (a partial one) would fetch it from origin by
# id, `refs/pull/*` and the fork network's objects included. grep reads to the end (no -q), so rev-list never dies of
# a closed pipe, which pipefail would read as no.
on_branch() {
  git -C "$repo" rev-list --remotes=origin | grep -xF "$1" > /dev/null
}

# Whether the commit id $1 is on origin/main in the clone, read the same way.
on_main() {
  git -C "$repo" rev-list refs/remotes/origin/main | grep -xF "$1" > /dev/null
}

# The distro's memory and the kernel's count of out-of-memory kills, read by `mutants` (and, for the count, by
# `scripts/contain.sh`'s unprivileged run).
meminfo=/proc/meminfo
# The memory allowed a job past the first, in kB (applied per R-369): 8 GiB, the per-process cap on a test process
# (`.cargo/mutants.toml`'s `prlimit --as`; R-348, REQ-VAL-181, R-905). An allowance, not a bound on what N jobs use:
# each job's nextest runs up to nproc capped test processes at once (`[profile.mutants]` sets no test-threads), and
# rustc and cargo run outside the cap. The guard is the out-of-memory count: a run during which the kernel's killer
# ran fails (`scripts/contain.sh`'s unprivileged run).
job_kib=8388608
# The warm build cache's bound: cleared before a run when it holds more than this, in kB. 24 GiB: twice a mutants run's
# whole target measured on the Mac (TASK-M0-66's, 11328516 kB, 17 Nov 2026), so one toolchain's and lockfile's
# artifacts and those its later commits leave behind fit, and no more (applied per R-369).
cache_cap_kib=25165824

# The workload on stdin, checked before anything runs. Every value reaches cargo as one argument, never a shell's word;
# the checks keep what may be asked to the criterion's shape: full commit ids, crates of the workspace's crates/ but
# the two no job on the PC builds (gui, prin; `win-gpu.yml`'s paths), and so never xtask, whose job queue's mutants
# stay on GitHub's runners (R-912); a timeout of whole seconds; a filterset of the characters nextest's names,
# operators and `=`, `/` matchers need; 1 to 4 jobs; and a shard as cargo mutants numbers them, from 0.
mutants() {
  local commit base packages timeout filter jobs="" shard="" p start code wall results="$HOME/results" dir="$work/mutants"
  local cache="$HOME/target" key have size state mem
  local -a pkgs args tuning
  local sha='^[0-9a-f]{40}$' name='^[a-z][a-z0-9_-]*$' secs='^[1-9][0-9]{0,3}$' set='^[A-Za-z0-9_:=()|&!/.~ -]+$'
  local ways='^[1-4]$' part='^(0|[1-9][0-9]?)/([1-9][0-9]?)$'
  { read -r commit && read -r base && read -r packages && read -r timeout && IFS= read -r filter; } ||
    refuse "the workload is five lines on stdin: commit, base, packages, timeout, filterset"
  # The two optional lines: absent or empty, each takes its default.
  read -r jobs || true
  read -r shard || true
  [[ $commit =~ $sha ]] || refuse "the commit '$commit' is not a full commit id"
  [[ $base =~ $sha ]] || refuse "the base '$base' is not a full commit id"
  [[ $timeout =~ $secs ]] || refuse "the timeout '$timeout' is not 1 to 9999 seconds"
  [[ $filter =~ $set ]] || refuse "the filterset '$filter' holds a character it may not"
  [ -n "$jobs" ] || jobs=1
  [[ $jobs =~ $ways ]] || refuse "the jobs '$jobs' are not 1 to 4"
  if [ -n "$shard" ]; then
    [[ $shard =~ $part ]] && [ "${BASH_REMATCH[1]}" -lt "${BASH_REMATCH[2]}" ] ||
      refuse "the shard '$shard' is not k/n, k from 0 to n-1 and n from 1 to 99"
    tuning+=(--shard "$shard")
  fi
  # More than one job runs that many builds and test runs at once: refused where the distro's memory is under the
  # allowance of 8 GiB a job (above), which is a floor against a plainly too small VM, not a proof that N jobs fit;
  # an out-of-memory kill fails the run whatever this says. One job is the footprint the measured run had (run
  # 43973677779).
  if [ "$jobs" -gt 1 ]; then
    mem=$(awk '$1 == "MemTotal:" { print $2 }' "$meminfo" 2> /dev/null || true)
    [[ $mem =~ ^[0-9]+$ ]] || refuse "-j $jobs: the distro's memory can't be read from $meminfo"
    [ "$mem" -ge $((jobs * job_kib)) ] ||
      refuse "-j $jobs needs $((jobs * job_kib)) kB, 8 GiB a job, and the distro has $mem kB"
  fi
  read -ra pkgs <<< "$packages"
  [ "${#pkgs[@]}" -gt 0 ] || refuse "no package named"
  for p in "${pkgs[@]}"; do
    [[ $p =~ $name ]] || refuse "the package '$p' is not a crate's name"
    case $p in gui | prin) refuse "the package '$p' is one no job on the PC builds" ;; esac
    args+=(--package "$p")
  done
  # Nothing is fetched: what runs is what the clone already holds, which `trust` has checked.
  git -C "$repo" merge-base --is-ancestor "$base" "$commit" || refuse "the base $base is not an ancestor of $commit"
  rm -rf "$dir" "$work/diff.patch"
  git -C "$repo" worktree prune
  git -C "$repo" worktree add --quiet --detach "$dir" "$commit"
  [ "$(git -C "$dir" rev-parse HEAD)" = "$commit" ] || refuse "$dir is not at $commit"
  for p in "${pkgs[@]}"; do
    [ -f "$dir/crates/$p/Cargo.toml" ] || refuse "the package '$p' is no crate under crates/ at $commit"
  done
  git -C "$repo" diff "$base" "$commit" > "$work/diff.patch"
  [ -s "$work/diff.patch" ] || refuse "the diff $base..$commit is empty"
  # An earlier run's results go only once every check has passed.
  rm -rf "$results"
  mkdir -p "$results"
  mv "$work/diff.patch" "$results/diff.patch"
  # The warm build cache (applied per R-369): ~/target, kept between dispatches, this verb's alone. Its key is the
  # commit's toolchain and lockfile, as git holds them; a cache of another key, or of none, is cleared, as is one
  # past its bound. At -j 1 the run builds in it. At more, cargo mutants' copies of the tree may not share a target:
  # each copy's tests would run the binaries another copy built (measured on a toy crate at -j 2, PR #556: two
  # missed mutants counted caught); so each copy builds in a target of its own, and the cache is left as it is, key and
  # all, for the next -j 1 run.
  key="toolchain $(git -C "$repo" rev-parse -q --verify "$commit:rust-toolchain.toml" || echo none)"
  key+=" lockfile $(git -C "$repo" rev-parse -q --verify "$commit:Cargo.lock" || echo none)"
  # cargo mutants' copies of the tree, and whatever else the run puts in a temporary folder, go in one folder of
  # this verb's, emptied before each run and after: a run stopped by a signal (the job's limit, a cancel) ends its
  # namespace with SIGKILL, before cargo mutants can remove its copies, a target in each at -j N > 1.
  rm -rf "$work/tmp"
  mkdir -p "$work/tmp"
  export TMPDIR="$work/tmp"
  export PRIN_CONTAIN=unprivileged
  if [ "$jobs" -eq 1 ]; then
    have=$(cat "$cache/.prin-cache-key" 2> /dev/null || true)
    size=$(du -sk "$cache" 2> /dev/null | cut -f1 || true)
    if [ ! -d "$cache" ]; then
      state="miss: none"
    elif [ "$have" != "$key" ]; then
      state="cleared: its key was '${have:-none}'"
    elif [ "${size:-0}" -gt "$cache_cap_kib" ]; then
      state="cleared: $size kB, past its bound of $cache_cap_kib kB"
    else
      state="hit: $size kB"
    fi
    case $state in cleared:*) rm -rf "$cache" ;; esac
    mkdir -p "$cache"
    echo "$key" > "$cache/.prin-cache-key"
    export CARGO_TARGET_DIR="$cache"
  else
    state="unused at -j $jobs: each copy builds a target of its own"
    unset CARGO_TARGET_DIR
  fi
  bash "$repo/scripts/contain.sh" check 2>&1 | tee "$results/contain-check.log"
  {
    echo "commit $commit"
    echo "base $base"
    echo "cpu $(grep -m 1 '^model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ *//')"
    echo "cores $(nproc)"
    echo "memory $(grep -m 1 '^MemTotal' "$meminfo" | awk '{ print $2, $3 }')"
    echo "load at start $(cat /proc/loadavg)"
    echo "jobs $jobs"
    echo "shard ${shard:-none}"
    echo "cache key $key"
    echo "cache $state"
  } > "$results/host.txt"
  start=$(date +%s)
  set +e
  (cd "$dir" && NEXTEST_PROFILE=mutants bash "$repo/scripts/contain.sh" run cargo mutants --in-diff \
    "$results/diff.patch" "${args[@]}" -j "$jobs" ${tuning[@]+"${tuning[@]}"} --timeout "$timeout" -o "$results" \
    -- -E "$filter") 2>&1 | tee "$results/mutants.log"
  code=${PIPESTATUS[0]}
  set -e
  wall=$(($(date +%s) - start))
  {
    echo "load at end $(cat /proc/loadavg)"
    echo "cache after $(du -sk "$cache" 2> /dev/null | cut -f1) kB"
  } >> "$results/host.txt"
  rm -rf "$work/tmp"
  echo "wall_seconds $wall" > "$results/wall.txt"
  cat "$results/host.txt" "$results/wall.txt"
  for p in caught missed timeout unviable; do
    echo "$p $(wc -l 2> /dev/null < "$results/mutants.out/$p.txt" || echo none)"
  done
  # cargo mutants' 2 (a mutant missed) and 3 (one timed out) are the run's results, kept in mutants.out; any other
  # failure is the run's own.
  echo "wsl-cpu: cargo mutants exited $code"
  case $code in
    0 | 2 | 3) return 0 ;;
    *) return "$code" ;;
  esac
}

# The workload's commits are the owner's: the commit on one of origin's branches, the base on origin/main, and each
# commit between them that origin/main doesn't hold signed with the owner's key, as the PC's signed-commit gate
# requires of what runs there (R-711 item 6). origin/main's own commits are the chain that gate already trusts.
trust() {
  local commit base c list signers="$repo/.github/allowed_signers" sha='^[0-9a-f]{40}$'
  local -a verify=(git -C "$repo" -c gpg.ssh.allowedSignersFile="$signers" -c gpg.ssh.program=ssh-keygen)
  { read -r commit && read -r base; } || refuse "the workload's commit and base are its first two lines on stdin"
  [[ $commit =~ $sha ]] || refuse "the commit '$commit' is not a full commit id"
  [[ $base =~ $sha ]] || refuse "the base '$base' is not a full commit id"
  [ -s "$signers" ] || refuse "no signing key is pinned in $signers"
  on_branch "$commit" || refuse "the commit $commit is on none of origin's branches (refs/pull is never fetched)"
  on_main "$base" || refuse "the base $base is not on origin/main"
  git -C "$repo" merge-base --is-ancestor "$base" "$commit" || refuse "the base $base is not an ancestor of $commit"
  list=$(git -C "$repo" rev-list "$commit" --not "$base" refs/remotes/origin/main) ||
    refuse "the commits from $base to $commit can't be listed"
  for c in $list; do
    # verify-commit fails unless the signature is good and its key a pinned one (%G? is G): no signature, another
    # key's, a bad one, or a GPG one, as GitHub's web-flow merges carry, are each refused.
    "${verify[@]}" verify-commit "$c" || refuse "the commit $c carries no good signature by the owner's key"
    echo "signed by the owner's key: $c"
  done
  echo "trust: $base..$commit is the owner's"
}

verb=${1:-}
[ "$#" -gt 0 ] && shift
case "$verb" in
  identity)
    whoami
    id
    uname -r
    grep '^PRETTY_NAME=' /etc/os-release
    ;;
  isolation) isolation ;;
  address) hostname -I | awk '{ print $1 }' ;;
  gateway) gateway "$@" ;;
  listen) listen "$@" ;;
  connect) connect "$@" ;;
  check) PRIN_CONTAIN=unprivileged bash "$repo/scripts/contain.sh" check ;;
  parity) parity "$@" ;;
  compare) python3 "$repo/scripts/wsl/parity.py" compare - "$work/parity.tsv" ;;
  cpu) cpu ;;
  trust) trust ;;
  mutants) mutants ;;
  results) tar -C "$HOME/results" -cf - . ;;
  *)
    echo "usage: inside.sh identity|isolation|address|gateway|listen|connect|check|parity|compare|cpu|trust|mutants|results" >&2
    exit 2
    ;;
esac
