#!/usr/bin/env bash
# The PR tier's product tests run on the changed crates and their reverse dependencies (the owner's CI model, approved
# 9 Nov 2026: "Product tests: changed crates + reverse deps (ci_quick_crates.py selection)"). This script names them.
#
#   .github/scripts/ci_tier.sh decide   In `ci.yml`'s `changes` job, after `scripts/ci_changes.sh detect`: writes to
#                                       $GITHUB_OUTPUT (and prints) `packages`, the tests' `-p` arguments, and
#                                       `doc_packages`, those of them with a library, the doctests' arguments, which can
#                                       be none.
#
# On a pull request the packages are `.github/scripts/ci_quick_crates.py`'s, from the changed paths
# `scripts/ci_changes.sh paths` prints; on any other event, or when the paths or the crates cannot be read, every crate
# (`--workspace`), which fails towards every test. The classifier's tier decides whether the tests run at all (`ci.yml`'s
# `tests` job); this script decides only which crates they cover.
set -euo pipefail

here=$(dirname "${BASH_SOURCE[0]}")

output() { # output <key> <value>
  echo "$1=$2"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "$1=$2" >> "$GITHUB_OUTPUT"
  fi
}

# The changed crates and their reverse dependencies, as cargo arguments: `--workspace` if the change cannot be read,
# which fails towards every test. Sets `packages` and `doc_packages`, those of them with a library, which
# `cargo test --doc` needs and which can be none.
changed_packages() {
  local changes="${CI_CHANGES:-$here/../../scripts/ci_changes.sh}" crates="" libraries="" paths crate
  local script="$here/ci_quick_crates.py" workspace="${CI_ROOT:-.}"
  paths=$("$changes" paths) \
    && crates=$(python3 -I "$script" --root "$workspace" <<< "$paths") \
    && libraries=$(python3 -I "$script" --root "$workspace" --library <<< "$paths") \
    || crates=""
  if [ -z "$crates" ]; then
    packages=--workspace
    doc_packages=--workspace
    return
  fi
  packages=""
  doc_packages=""
  for crate in $crates; do
    packages="$packages -p $crate"
  done
  packages=${packages# }
  for crate in $libraries; do
    doc_packages="$doc_packages -p $crate"
  done
  doc_packages=${doc_packages# }
}

decide() {
  packages=--workspace
  doc_packages=--workspace
  if [ "${GITHUB_EVENT_NAME:-}" = pull_request ]; then
    changed_packages
  fi
  output packages "$packages"
  output doc_packages "$doc_packages"
}

case "${1:-}" in
  decide) decide ;;
  *)
    echo "usage: .github/scripts/ci_tier.sh decide" >&2
    exit 2
    ;;
esac
