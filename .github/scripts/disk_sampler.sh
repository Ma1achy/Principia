#!/usr/bin/env bash
# The `shell:` of `cargo xtask ci` in `xtask-ci` (.github/workflows/ci.yml): runs the step's script exactly as the
# default shell would (`bash -e {0}`), samples the runner's disk beside it, and exits with the script's own status. No
# gate reads what it measures, and nothing it does can change that status: it has no `set -e`, and every sampling
# failure, a write to a full disk among them, is ignored. It is here and not in `scripts/` because `.github/` is full CI
# whole, as a step's shell must be (R-662).
#
# Each sample goes to $RUNNER_TEMP/runner-disk.log, which the job uploads. A runner that dies uploads nothing, so a
# coarse part also goes to the step's log: the first sample, every fifth, and every sample once under 2 GiB is free.
# Bounded: at most 480 samples of at most 40 `du` lines, more than the runner's 6 h job limit at the 60 s interval.
set -u

script=$1
out="${RUNNER_TEMP:-/tmp}/runner-disk.log"
seconds="${RUNNER_DISK_SAMPLE_SECONDS:-60}"
log_every=5
low_kb=$((2 * 1024 * 1024))
max_samples=480

sampler() {
  exec 2> /dev/null
  # `$!`, not a variable set after the `sleep` starts: a TERM is handled between two commands, so it can fall between
  # those two, where a variable would not name the `sleep` yet and it would be left running.
  trap 'kill "${!:-}"; exit 0' TERM
  local n avail text
  for ((n = 1; n <= max_samples; n++)); do
    avail=$(df -Pk / | awk 'NR == 2 { print $4 }')
    text="== sample $n, +$SECONDS s, avail_kb=$avail, $(date -u +%H:%M:%S)
$(df -h /)
$(du -sh target/debug/deps target/tmp/* /tmp | head -n 40)
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

sampler &
sampler_pid=$!

bash -e "$script"
status=$?

# The sampler ends at its trap, once the command in hand is done, or at the KILL; it never outlives the step. What it
# leaves running at a KILL, a `du` in hand, writes to the sample's own pipe and /dev/null, never to the step's output.
kill -TERM "$sampler_pid" 2> /dev/null
for _ in {1..15}; do
  kill -0 "$sampler_pid" 2> /dev/null || break
  sleep 1
done
kill -KILL "$sampler_pid" 2> /dev/null
wait "$sampler_pid" 2> /dev/null

awk 'match($0, /avail_kb=[0-9]+/) {
       kb = substr($0, RSTART + 9, RLENGTH - 9) + 0; n++
       if (n == 1 || kb < low) low = kb
     }
     END {
       if (n) printf "::notice title=Runner disk::lowest free %d MiB over %d samples\n", low / 1024, n
     }' "$out" 2> /dev/null

exit "$status"
