#!/usr/bin/env bash
# The `shell:` of `cargo xtask ci` in `xtask-ci`, and of the heavy steps of the `ci` and `ci-release` shards,
# `ci-workspace` and `gpu-kernel` (.github/workflows/ci.yml): runs the step's script exactly as the
# default shell would (`bash -e {0}`), samples the runner's disk beside it, and exits with the script's own status. No
# gate reads what it measures, and nothing it does can change that status: it has no `set -e`, and every sampling
# failure, a write to a full disk among them, is ignored. It is here and not in `scripts/` because `.github/` is full CI
# whole, as a step's shell must be (R-662).
#
# Each sample goes to $RUNNER_TEMP/runner-disk.log, which `xtask-ci`, `ci-workspace` and `ci-release` upload. A runner
# that dies uploads nothing, so a coarse part also goes to the step's log: the first sample, every fifth, and every
# sample once under 2 GiB is free.
# Bounded: at most 480 samples of at most 40 `du` lines, more than the runner's 6 h job limit at the 60 s interval.
#
# Beside it, `watch` names a stall of the whole runner, which froze the sampler itself in `ci-workspace` (job
# 126417229272): the sampler's `df`, `du` and its log depend on the disk, so what it reads from /proc stops when the
# disk does. `watch` is a process of its own that writes $RUNNER_DISK_TMPFS/runner-pressure.log, on a tmpfs, and after
# its first sample it reads and naps with shell builtins, so it forks nothing and opens nothing on the disk. It still
# runs from the disk: bash and libc are file-backed, so a stall in reclaim or page-in can stop `watch` too, and a gap in
# its samples is possible and is evidence in itself. `device_watch`, which has to run a command, is a process apart, so
# a stuck `dmsetup` stops only itself. The job sets RUNNER_DISK_TMPFS; the variable goes no further than this script, so
# that the tests the step runs, which run this script on scratch folders, stay off the real tmpfs and never ask for
# sudo. Without a usable folder there, `watch` writes beside $out, and without the variable `device_watch` doesn't run.
set -u

script=$1
out="${RUNNER_TEMP:-/tmp}/runner-disk.log"
seconds="${RUNNER_DISK_SAMPLE_SECONDS:-60}"
log_every=5
low_kb=$((2 * 1024 * 1024))
max_samples=480
proc=/proc
watch_seconds="${RUNNER_PRESSURE_SAMPLE_SECONDS:-5}"
# The runner's 6 h job limit at the 5 s interval: about 2 MiB of tmpfs.
max_watch_samples=4320
tmpfs="${RUNNER_DISK_TMPFS:-}"
unset RUNNER_DISK_TMPFS
watch_dir=$tmpfs
if ! [ -d "$watch_dir" ] || ! [ -w "$watch_dir" ]; then
  watch_dir="${RUNNER_TEMP:-/tmp}"
fi
watch_out="$watch_dir/runner-pressure.log"

# Sets `proc_lines` to the runner's load: no subshell, and past the first call no command, only builtins.
read_proc() {
  local kind line=
  : "${cpus:=$(nproc)}"
  read -r line < "$proc/loadavg"
  proc_lines="nproc ${cpus:-none}
loadavg ${line:-none}"
  for kind in cpu memory io; do
    while IFS= read -r line; do
      proc_lines+=$'\n'"$kind $line"
    done < "$proc/pressure/$kind"
  done
  while IFS= read -r line; do
    case $line in
      Dirty:* | Writeback:*) proc_lines+=$'\n'"$line" ;;
    esac
  done < "$proc/meminfo"
}

# The device-mapper's state, the copy-on-write root's fill among it, where sudo allows it. `timeout` runs under `sudo`,
# as root, because `dmsetup` runs as root and this user's `timeout` could not signal it. The read holds none of the
# step's streams. The KILL of the group at the step's end stops this loop, so no read starts after the step, but it
# doesn't reach root processes, so `sudo`, `timeout` and `dmsetup` may outlive the step by up to the limit. `sudo`
# itself runs before any timer starts, so a stall there is unbounded, but it costs only these samples, and shows as a
# gap in them. A process stuck in the kernel in D state survives everything, the limit included. What root and `timeout` do
# here comes from sudo's and coreutils' master sources, not `ubuntu-24.04`'s; CI is the first real run.
device_watch() {
  local n text
  for ((n = 1; n <= max_samples; n++)); do
    text=$(sudo -n timeout 10 dmsetup status 2>&1)
    printf '== device %d, +%d s\n%s\n' "$n" "$SECONDS" "$text" >> "$watch_out"
    sleep "$seconds"
  done
}

watch() {
  exec > /dev/null 2>&1 < /dev/null
  [ -z "$tmpfs" ] || device_watch &
  local n fifo="$watch_dir/runner-pressure.$$.fifo"
  mkfifo "$fifo" && exec 9<> "$fifo"
  rm -f "$fifo"
  for ((n = 1; n <= max_watch_samples; n++)); do
    read_proc
    printf '== watch %d, +%d s, epoch %s\n%s\n' "$n" "$SECONDS" "${EPOCHSECONDS:-}" "$proc_lines" >> "$watch_out"
    # A timed read of a pipe nothing writes to: a nap with no fork. Its status tops 128 on the timeout (bash 3.2's
    # doesn't, so there the nap is longer); anything else means there is no pipe, and `sleep` keeps this from spinning.
    read -r -t "$watch_seconds" <&9
    (($? > 128)) || sleep "$watch_seconds"
  done
}

sampler() {
  exec 2> /dev/null
  # The trap runs between two commands, never during one, so it waits for a command in hand to end; the main shell's
  # KILL of the group is what bounds that. `$!`, not a variable set after the `sleep` starts: a TERM can fall between
  # those two commands, where a variable would not name the `sleep` yet and it would be left running.
  trap 'kill "${!:-}"; exit 0' TERM
  local n avail text
  for ((n = 1; n <= max_samples; n++)); do
    avail=$(df -Pk / | awk 'NR == 2 { print $4 }')
    read_proc
    text="== sample $n, +$SECONDS s, avail_kb=$avail, $(date -u +%H:%M:%S)
$(df -h /)
$proc_lines
$(du -sh target/debug/deps target/release/deps target/tmp/* /tmp | head -n 40)
$(free -m)"
    printf '%s\n' "$text" >> "$out"
    if ((n % log_every == 1 || ${avail:-0} < low_kb)); then
      printf '%s\n' "$text"
    fi
    # The one child that inherits the step's streams: it gets none, so even a `sleep` nothing ended holds no pipe open,
    # and in Actions a held pipe can hold the step open.
    sleep "$seconds" > /dev/null 2>&1 &
    wait "$!"
  done
}

# In its own process group (`set -m` only for this start), so the KILL below reaches whatever it has in hand.
set -m
sampler &
sampler_pid=$!
set +m

# In a group of its own too, with `device_watch`: the KILL below stops that loop, not the root processes of its read.
set -m
watch &
watch_pid=$!
set +m

bash -e "$script"
status=$?

# `watch` writes one sample at a time and has nothing to finish.
kill -KILL -- "-$watch_pid" 2> /dev/null
wait "$watch_pid" 2> /dev/null

# Bounded here, not by the sampler: its trap runs only once the command in hand is done (a `df` or `du`, or the shell
# retrying a fork), so a sampler still there after at most 2 s gets a KILL to its whole group, that command and the
# nap among them. A sampler gone by then went at its trap, between two commands, which ended the nap and left nothing.
kill -TERM "$sampler_pid" 2> /dev/null
end=$((SECONDS + 2))
while kill -0 "$sampler_pid" 2> /dev/null && ((SECONDS < end)); do
  sleep 0.1
done
kill -0 "$sampler_pid" 2> /dev/null && kill -KILL -- "-$sampler_pid" 2> /dev/null
wait "$sampler_pid" 2> /dev/null

awk 'match($0, /avail_kb=[0-9]+/) {
       kb = substr($0, RSTART + 9, RLENGTH - 9) + 0; n++
       if (n == 1 || kb < low) low = kb
     }
     END {
       if (n) printf "::notice title=Runner disk::lowest free %d MiB over %d samples\n", low / 1024, n
     }' "$out" 2> /dev/null

exit "$status"
