#!/usr/bin/env bash
# Cloud-session setup (R-346): installs on a Linux machine exactly what CI's Linux jobs install, so that a cloud session
# builds and tests as CI does, then runs `python3 plan/check_plan.py` as a smoke test. plan/OPERATIONS.md § "Start
# here" says when to run it.
#
# Every pin is read from the files CI reads, never copied here (R-346, the human's (C)): the Rust toolchains from the
# `dtolnay/rust-toolchain@<ref>` steps and their `components:`, and from the repository's root `rust-toolchain.toml`
# (or `rust-toolchain`), which CI's bare `rustup toolchain install` step installs; the cargo tools and their versions
# from the `tool:` lines of the `taiki-e/install-action` steps; the apt packages from the `apt-get install` lines; the
# Python version from the `actions/setup-python` steps' `python-version:`; the Python packages from the `pip install`
# lines; and PRIN_GPU_BACKEND from the jobs' `env:`. Only jobs that run on `ubuntu-*` are read. A step that installs
# something by a means this script doesn't know stops it, naming the step, so CI cannot gain an install the script
# misses. `xtask/tests/cloud_setup.rs` checks that this script's plan and CI agree, and that this file holds no version.
#
# How each item is installed (R-346, R-347): the toolchains through rustup, the apt packages through apt-get, the Python
# packages through pip. cargo-nextest comes from its official prebuilt installer (get.nexte.st) and cargo-mutants through
# cargo-binstall (prebuilt; cargo-binstall itself from its official prebuilt installer when missing), each at CI's
# pinned version, and each falls back to `cargo install --locked <tool>@<version>` only if its download fails. Any other
# cargo tool is built with `cargo install --locked`. The downloads need network access to get.nexte.st and GitHub's
# releases.
#
# Usage:
#   scripts/cloud-setup.sh            install, then run the smoke test
#   scripts/cloud-setup.sh --dry-run  print the plan, one item per line (`<kind> <name> <version> <method>`), install
#                                     nothing
#
# Idempotent: an item already installed at the pinned version is skipped. apt runs as root, or through sudo. Sourced
# (`. scripts/cloud-setup.sh`), it defines its functions and installs nothing.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOWS="$ROOT/.github/workflows"

DRY_RUN=0
case "${1:-}" in
  --dry-run) DRY_RUN=1 ;;
  "") ;;
  *)
    echo "usage: scripts/cloud-setup.sh [--dry-run]" >&2
    exit 2
    ;;
esac

say() { echo "cloud-setup: $*" >&2; }
die() {
  say "error: $*"
  exit 1
}

# CI's Linux jobs, read as records, one per line:
#   R <ref>            a `dtolnay/rust-toolchain@<ref>` step
#   C <ref> <comp>     a component that step's `components:` names
#   F                  a bare `rustup toolchain install`, which installs what the root toolchain file pins
#   T <name> <ver|->   a cargo tool from a `tool:` line, `<name>@<ver>` or unpinned
#   A <pkg>            an `apt-get install` package
#   P <ver>            an `actions/setup-python` step's `python-version:`
#   Y <pkg>            a `pip install` package
#   E <var> <value>    PRIN_GPU_BACKEND from an `env:`
#   U <where>: <text>  a step that installs by a means this script doesn't know
ci_records() {
  local files
  files=$(find "$WORKFLOWS" -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) | sort)
  [ -n "$files" ] || die "no workflow files under $WORKFLOWS"
  # shellcheck disable=SC2086 # the workflow paths have no spaces; word splitting passes each as one file
  awk -v q="'" '
    function trim(s) { sub(/^[ \t]+/, "", s); sub(/[ \t]+$/, "", s); return s }
    function unquote(s) { s = trim(s); gsub("^[\"" q "]|[\"" q "]$", "", s); return s }
    function indent(s,   t) { t = s; sub(/^ +/, "", t); return length(s) - length(t) }
    function emit(s) { buf = buf s "\n" }
    function flush_job() { if (linux) printf "%s", buf; buf = ""; linux = 0; tc = ""; run_ind = -1 }
    # The words after `key` in `cmd`, up to the end of that command: options are skipped.
    function words_after(cmd, key, kind,   i, n, w, rest) {
      i = index(cmd, key)
      rest = substr(cmd, i + length(key))
      n = split(rest, w, /[ \t]+/)
      for (j = 1; j <= n; j++) {
        if (w[j] == "") continue
        if (w[j] ~ /^(&&|\|\||;|\||\\)$/) break
        if (w[j] ~ /^-/) continue
        emit(kind " " w[j])
      }
    }
    function scan_command(cmd) {
      cmd = trim(cmd)
      if (cmd == "" || substr(cmd, 1, 1) == "#") return
      if (index(cmd, "apt-get install") > 0) { words_after(cmd, "apt-get install", "A"); return }
      if (index(cmd, "pip install") > 0) { words_after(cmd, "pip install", "Y"); return }
      if (cmd ~ /(^|[;&|] *)rustup toolchain install *([;&|]|$)/) { emit("F"); return }
      if (cmd ~ /cargo install|cargo binstall|rustup (component|target) add|rustup toolchain install|(^|[^-])apt install|snap install|brew install|npm (install|ci)|curl |wget /) {
        emit("U " FILENAME ":" FNR ": " cmd)
      }
    }
    FNR == 1 { flush_job(); in_jobs = 0 }
    {
      line = $0
      t = trim(line)
      if (t == "") next
      ind = indent(line)
      if (run_ind >= 0) {
        if (ind > run_ind) { scan_command(t); next }
        run_ind = -1
      }
      if (substr(t, 1, 1) == "#") next
      if (ind == 0) { flush_job(); in_jobs = (t == "jobs:"); next }
      if (!in_jobs) next
      if (ind == 2) { flush_job(); next }
      if (substr(t, 1, 2) == "- ") { tc = ""; t = substr(t, 3); t = trim(t); step_ind = ind }
      else if (ind <= step_ind) { tc = "" }
      if (t ~ /^runs-on:/) {
        v = unquote(substr(t, 9))
        if (v ~ /^ubuntu-/) linux = 1
        else if (v !~ /^(macos|windows)-/) print "U " FILENAME ":" FNR ": " t
        next
      }
      if (t ~ /^uses:/) {
        v = unquote(substr(t, 6))
        if (v ~ /^dtolnay\/rust-toolchain@/) { tc = substr(v, index(v, "@") + 1); emit("R " tc) }
        else if (v ~ /^(actions\/checkout|actions\/cache|actions\/cache\/restore|actions\/cache\/save|actions\/upload-artifact|actions\/download-artifact|Swatinem\/rust-cache|taiki-e\/install-action|actions\/setup-python)@/) { }
        else emit("U " FILENAME ":" FNR ": " t)
        next
      }
      if (t ~ /^components:/) {
        if (tc == "") { emit("U " FILENAME ":" FNR ": " t); next }
        n = split(unquote(substr(t, 12)), c, /[ ,]+/)
        for (j = 1; j <= n; j++) if (c[j] != "") emit("C " tc " " c[j])
        next
      }
      if (t ~ /^toolchain:/ && tc != "") { emit("U " FILENAME ":" FNR ": " t); next }
      if (t ~ /^tool:/) {
        n = split(unquote(substr(t, 6)), c, /[ ,]+/)
        for (j = 1; j <= n; j++) {
          if (c[j] == "") continue
          at = index(c[j], "@")
          if (at > 0) emit("T " substr(c[j], 1, at - 1) " " substr(c[j], at + 1))
          else emit("T " c[j] " -")
        }
        next
      }
      if (t ~ /^python-version:/) { emit("P " unquote(substr(t, 16))); next }
      if (t ~ /^PRIN_GPU_BACKEND:/) { emit("E PRIN_GPU_BACKEND " unquote(substr(t, 18))); next }
      if (t ~ /^run:/) {
        v = trim(substr(t, 5))
        if (v ~ /^[|>][-+]?$/) run_ind = ind
        else scan_command(v)
        next
      }
    }
    END { flush_job() }
  ' $files | sort -u
}

# The root toolchain file, as records: `channel <c>`, `profile <p>`, `component <c>`, `target <t>`. CI's bare
# `rustup toolchain install` step reads the same file.
toolchain_file() {
  local file=""
  if [ -f "$ROOT/rust-toolchain.toml" ]; then
    file="$ROOT/rust-toolchain.toml"
  elif [ -f "$ROOT/rust-toolchain" ]; then
    file="$ROOT/rust-toolchain"
  fi
  [ -n "$file" ] || return 0
  awk '
    function trim(s) { sub(/^[ \t]+/, "", s); sub(/[ \t]+$/, "", s); return s }
    function unquote(s) { s = trim(s); gsub(/^"|"$/, "", s); return s }
    function list(kind, v,   n, c, j) {
      gsub(/[][]/, "", v); n = split(v, c, /,/)
      for (j = 1; j <= n; j++) { x = unquote(c[j]); if (x != "") print kind " " x }
    }
    {
      t = trim($0)
      if (t == "" || substr(t, 1, 1) == "#") next
      if (pending != "") { acc = acc " " t; if (index(t, "]") > 0) { list(pending, acc); pending = "" }; next }
      if (t == "[toolchain]") next
      if (index(t, "=") == 0) { if (seen++ == 0) print "channel " t; else print "unknown " t; next }
      key = trim(substr(t, 1, index(t, "=") - 1)); v = trim(substr(t, index(t, "=") + 1))
      if (key == "channel" || key == "profile") { print key " " unquote(v); next }
      if (key == "components" || key == "targets") {
        kind = (key == "components") ? "component" : "target"
        if (index(v, "]") > 0) list(kind, v); else { pending = kind; acc = v }
        next
      }
      print "unknown " t
    }
  ' "$file"
}

# How a cargo tool is installed (R-347): `<route>` or `<route>,fallback:cargo-install`.
tool_method() {
  case "$1" in
    cargo-nextest) echo "prebuilt:get.nexte.st,fallback:cargo-install" ;;
    cargo-mutants) echo "prebuilt:cargo-binstall,fallback:cargo-install" ;;
    *) echo "cargo-install" ;;
  esac
}

# How an item of the plan is installed: the fourth word of each plan line.
method() {
  case "$1" in
    toolchain | profile | component | target) echo rustup ;;
    apt) echo apt-get ;;
    pip) echo pip ;;
    python) echo check ;;
    env) echo export ;;
    cargo-tool) tool_method "$2" ;;
    *) die "no install method for $1 $2" ;;
  esac
}

# The plan, one item per line, `<kind> <name> <version> <method>` (`-` where there is no version), sorted: what this
# script installs and how, and what `--dry-run` prints.
plan() {
  local records unknown values tc
  records=$(ci_records) || return 1
  unknown=$(printf '%s\n' "$records" | sed -n 's/^U //p')
  [ -z "$unknown" ] || die "CI's Linux jobs install by a step this script doesn't know; teach it, then re-run:
$unknown"
  # One version of a tool or of Python: CI pinning two would leave the script nothing to choose by.
  values=$(printf '%s\n' "$records" | awk '$1 == "T" || $1 == "P" || $1 == "E" { print $1, $2 }' | sort | uniq -d)
  [ -z "$values" ] || die "CI's Linux jobs pin more than one version of: $values"
  tc=$(toolchain_file) || return 1
  unknown=$(printf '%s\n' "$tc" | sed -n 's/^unknown //p')
  [ -z "$unknown" ] || die "the root toolchain file has a key this script doesn't know: $unknown"
  if [ -n "$tc" ] && ! printf '%s\n' "$tc" | grep -q '^channel '; then die "the root toolchain file names no channel"; fi
  {
    printf '%s\n' "$records" | awk '
      $1 == "R" { print "toolchain action " $2; print "profile " $2 " minimal" }
      $1 == "C" { print "component " $2 "/" $3 " -" }
      $1 == "T" { print "cargo-tool " $2 " " $3 }
      $1 == "A" { print "apt " $2 " -" }
      $1 == "P" { print "python python " $2 }
      $1 == "Y" { print "pip " $2 " -" }
      $1 == "E" { print "env " $2 " " $3 }
    '
    [ -z "$tc" ] || printf '%s\n' "$tc" | awk '
      $1 == "channel" { ch = $2 } $1 == "profile" { pr = $2 }
      $1 == "component" { c[++nc] = $2 } $1 == "target" { g[++ng] = $2 }
      END {
        print "toolchain file " ch
        print "profile " ch " " (pr == "" ? "-" : pr)
        for (j = 1; j <= nc; j++) print "component " ch "/" c[j] " -"
        for (j = 1; j <= ng; j++) print "target " ch "/" g[j] " -"
      }
    '
  } | sort -u | while read -r kind name version; do
    echo "$kind $name $version $(method "$kind" "$name")"
  done
}

# cargo tools: the `taiki-e/install-action` steps' `tool:` lines, each at CI's version (`-`: unpinned), installed as
# `tool_method` says (R-347).
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

# get.nexte.st's name for this machine's platform.
nextest_platform() {
  case "$(uname -s)/$(uname -m)" in
    Linux/x86_64) echo linux ;;
    Linux/aarch64 | Linux/arm64) echo linux-arm ;;
    Darwin/*) echo mac ;;
    *) return 1 ;;
  esac
}

# cargo-nextest from its official prebuilt installer.
nextest_download() {
  local version="$1" platform
  [ "$version" != - ] || version=latest
  platform=$(nextest_platform) || {
    say "get.nexte.st has no build for $(uname -s)/$(uname -m)"
    return 1
  }
  mkdir -p "$CARGO_BIN" || return 1
  say "curl https://get.nexte.st/$version/$platform | tar zxf - -C $CARGO_BIN"
  curl --proto '=https' -LsSf "https://get.nexte.st/$version/$platform" | tar zxf - -C "$CARGO_BIN"
}

# A cargo tool through cargo-binstall, prebuilt only: binstall's own build-from-source strategy is off, so a failed
# download falls back to `cargo install --locked` here, never to an unlocked build.
binstall_download() {
  local spec="$1"
  [ "$2" = - ] || spec="$1@$2"
  if ! command -v cargo-binstall >/dev/null 2>&1; then
    say "installing cargo-binstall from its official prebuilt installer"
    curl --proto '=https' -LsSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
      | bash || return 1
  fi
  say "cargo binstall $spec"
  cargo binstall --no-confirm --disable-strategies compile "$spec"
}

# Installs cargo tool $1 at version $2: its prebuilt route, if it has one, and `cargo install --locked` only if that
# download fails. The build runs in the repository, so with the toolchain CI's jobs use there.
install_cargo_tool() {
  local tool="$1" version="$2" spec="$1" how
  [ "$version" = - ] || spec="$tool@$version"
  how=$(tool_method "$tool")
  case "$how" in
    prebuilt:get.nexte.st,*) nextest_download "$version" && return 0 ;;
    prebuilt:cargo-binstall,*) binstall_download "$tool" "$version" && return 0 ;;
  esac
  case "$how" in
    *fallback:cargo-install) say "$tool: the download failed; falling back to cargo install --locked $spec" ;;
  esac
  say "cargo install --locked $spec"
  (cd "$ROOT" && cargo install --locked "$spec")
}

# The plan's items of kind $1, as `<name> <version>`.
items() { printf '%s\n' "$PLAN" | awk -v k="$1" '$1 == k { print $2, $3 }'; }

apt_install() {
  local missing="" pkg
  for pkg in "$@"; do
    dpkg -s "$pkg" >/dev/null 2>&1 || missing="$missing $pkg"
  done
  [ -n "$missing" ] || return 0
  command -v apt-get >/dev/null 2>&1 || die "apt-get not found: install$missing by hand"
  if [ "$(id -u)" != 0 ] && [ -z "$SUDO" ]; then die "not root and no sudo: install$missing as root"; fi
  say "apt-get install$missing"
  # shellcheck disable=SC2086 # one word per package
  $SUDO apt-get update && $SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y $missing
}

# Installs toolchain $1 with profile $2 (`-`: rustup's default) and the components and targets the plan gives it.
install_toolchain() {
  local channel="$1" profile="$2" args="" comp target
  for comp in $(items component | awk -v ch="$channel" 'index($1, ch "/") == 1 { print substr($1, length(ch) + 2) }'); do
    args="$args --component $comp"
  done
  for target in $(items target | awk -v ch="$channel" 'index($1, ch "/") == 1 { print substr($1, length(ch) + 2) }'); do
    args="$args --target $target"
  done
  [ "$profile" = - ] || args="$args --profile $profile"
  say "rustup toolchain install $channel$args"
  # shellcheck disable=SC2086 # one word per option
  rustup toolchain install "$channel" --no-self-update $args
}

main() {
  PLAN=$(plan) || exit 1

  if [ "$DRY_RUN" = 1 ]; then
    printf '%s\n' "$PLAN"
    echo "smoke python3 plan/check_plan.py"
    exit 0
  fi

  [ "$(uname -s)" = Linux ] || say "warning: this is for a Linux machine; on $(uname -s) it installs only what it can"

  SUDO=""
  if [ "$(id -u)" != 0 ]; then
    if command -v sudo >/dev/null 2>&1; then SUDO="sudo"; fi
  fi

  # What a GitHub-hosted runner image has before any step runs, and CI's steps rely on.
  for cmd in git curl cc; do
    command -v "$cmd" >/dev/null 2>&1 || die "$cmd not found; the runner image CI uses has it (for cc, apt's build-essential)"
  done

  # apt packages: CI's "Install Mesa (lavapipe)" steps.
  APT=$(items apt | awk '{ print $1 }')
  if [ -n "$APT" ]; then
    # shellcheck disable=SC2086 # one word per package
    apt_install $APT
  fi

  # Rust. The runner image has rustup; a machine without it gets it, with no toolchain of its own.
  export PATH="$HOME/.cargo/bin:$PATH"
  if ! command -v rustup >/dev/null 2>&1; then
    say "installing rustup"
    curl --proto '=https' -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain none
  fi
  # `dtolnay/rust-toolchain@<ref>` steps: the action installs <ref> with the minimal profile and the step's
  # components, and makes it the default toolchain.
  for channel in $(items toolchain | awk '$1 == "action" { print $2 }'); do
    install_toolchain "$channel" "$(items profile | awk -v ch="$channel" '$1 == ch { print $2 }')"
    rustup default "$channel"
  done
  # The bare `rustup toolchain install` steps: the channel, profile, components and targets the root toolchain file
  # pins.
  for channel in $(items toolchain | awk '$1 == "file" { print $2 }'); do
    install_toolchain "$channel" "$(items profile | awk -v ch="$channel" '$1 == ch { print $2 }')"
  done

  # cargo tools: one already at CI's version is skipped; the rest go through install_cargo_tool (R-347).
  items cargo-tool | while read -r tool version; do
    sub="${tool#cargo-}"
    if [ "$version" = - ]; then
      if (cd "$ROOT" && cargo "$sub" --version >/dev/null 2>&1); then continue; fi
    elif (cd "$ROOT" && cargo "$sub" --version 2>/dev/null | awk -v v="$version" '{ for (i = 1; i <= NF; i++) if ($i == v) f = 1 } END { exit !f }'); then
      say "$tool $version: installed"
      continue
    fi
    install_cargo_tool "$tool" "$version"
  done

  # Python: the `actions/setup-python` steps' version, and the `pip install` steps' packages. xtask and the smoke test
  # run `python3`, so the packages go to it.
  PY_VERSION=$(items python | awk '{ print $2 }')
  command -v python3 >/dev/null 2>&1 || apt_install python3 python3-pip
  if [ -n "$PY_VERSION" ]; then
    have=$(python3 -c 'import sys; print("%d.%d" % sys.version_info[:2])')
    case "$have" in
      "$PY_VERSION" | "$PY_VERSION".*) ;;
      *) say "warning: python3 is $have; CI's jobs set up $PY_VERSION (plan/OPERATIONS.md § \"Start here\")" ;;
    esac
  fi
  python3 -m pip --version >/dev/null 2>&1 || python3 -m ensurepip --user >/dev/null 2>&1 || apt_install python3-pip
  for pkg in $(items pip | awk '{ print $1 }'); do
    if python3 -m pip show "$pkg" >/dev/null 2>&1; then
      say "$pkg: installed"
      continue
    fi
    say "pip install $pkg"
    python3 -m pip install "$pkg" \
      || python3 -m pip install --user "$pkg" \
      || python3 -m pip install --user --break-system-packages "$pkg"
  done

  # The GPU backend CI's Linux jobs test on (lavapipe, R-169).
  for pair in $(items env | awk '{ print $1 "=" $2 }'); do
    export "${pair?}"
    say "exported $pair for this run; to keep it, add to your shell profile: export $pair"
  done
  say "to keep cargo on PATH, add to your shell profile: export PATH=\"\$HOME/.cargo/bin:\$PATH\""

  # The smoke test.
  say "smoke test: python3 plan/check_plan.py"
  (cd "$ROOT" && python3 plan/check_plan.py)
}

# Run, unless sourced.
if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  main
fi
