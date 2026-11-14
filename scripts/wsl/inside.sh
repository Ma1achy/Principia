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
#                                    in its unprivileged mode, into ~/results (`wsl-cpu`, while its flag is set)
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
# selected above (lavapipe on both sides), under the `ci` profile CI's shards use, no retries, every test run.
parity() {
  local commit=$1 dir="$work/parity" start code
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
}

cpu() {
  local results="$HOME/results" code=0 tests mutants
  rm -rf "$results"
  mkdir -p "$results"
  cd "$repo"
  export CARGO_TARGET_DIR="$HOME/target" PRIN_CONTAIN=unprivileged
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
  results) tar -C "$HOME/results" -cf - . ;;
  *)
    echo "usage: inside.sh identity|isolation|address|gateway|listen|connect|check|parity|compare|cpu|results" >&2
    exit 2
    ;;
esac
