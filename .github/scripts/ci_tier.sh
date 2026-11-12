#!/usr/bin/env bash
# The PR tier's product tests run on the changed crates and their reverse dependencies (the owner's CI model, R-909,
# 9 Nov 2026: "Product tests: changed crates + reverse deps (ci_quick_crates.py selection)"). This script names them.
#
#   .github/scripts/ci_tier.sh decide   In `ci.yml`'s `changes` job, after `scripts/ci_changes.sh detect`: writes to
#                                       $GITHUB_OUTPUT (and prints) `packages`, the tests' `-p` arguments, and
#                                       `doc_packages`, those of them with a library, the doctests' arguments, which can
#                                       be none.
#
# On a pull request the packages are `.github/scripts/ci_quick_crates.py`'s, from the changed paths
# `scripts/ci_changes.sh paths` prints; on any other event, or when the paths or the crates cannot be read, every crate
# (`--workspace`), which fails towards every test; on a pull request whose only code-tier path is
# `plan/requirements.yaml`, only the tests that read it (R-980), the per-merge run running the rest on every merge. The
# classifier's tier decides whether the tests run at all (`ci.yml`'s `tests` job); this script decides only which tests
# they cover.
set -euo pipefail

here=$(dirname "${BASH_SOURCE[0]}")

output() { # output <key> <value>
  echo "$1=$2"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "$1=$2" >> "$GITHUB_OUTPUT"
  fi
}

# On a change whose only code-tier path is `plan/requirements.yaml` (`scripts/ci_changes.sh requirements`), the tests
# that read it and no other product test (R-980, refining R-677): each reader's package and `--test`, and a filterset
# naming exactly those binaries, since a `--test` name can be another selected package's too; no doctest reads it.
# Sets `read_by`, `packages` and `doc_packages` and succeeds; fails, setting nothing, on any other change, or when the
# readers cannot be read, which leaves the changed crates' selection.
readers() { # readers <changes script> <paths> <crates script> <workspace>
  local only ids id pkgs="" tests="" filter=""
  only=$("$1" requirements <<< "$2") && [ "$only" = requirements_only=true ] || return 1
  ids=$(python3 -I "$3" --root "$4" --readers) && [ -n "$ids" ] || return 1
  for id in $ids; do
    case "$pkgs " in *" -p ${id%%::*} "*) ;; *) pkgs="$pkgs -p ${id%%::*}" ;; esac
    case "$tests " in *" --test ${id#*::} "*) ;; *) tests="$tests --test ${id#*::}" ;; esac
    filter="$filter|binary_id(=$id)"
  done
  read_by=$ids
  packages="${pkgs# }$tests -E ${filter#|}"
  doc_packages=""
}

# The changed crates and their reverse dependencies, as cargo arguments: `--workspace` if the change cannot be read,
# which fails towards every test. Sets `packages` and `doc_packages`, those of them with a library, which
# `cargo test --doc` needs and which can be none.
changed_packages() {
  local changes="${CI_CHANGES:-$here/../../scripts/ci_changes.sh}" crates="" libraries="" paths crate read_by=""
  local script="$here/ci_quick_crates.py" workspace="${CI_ROOT:-.}"
  paths=$("$changes" paths) \
    && ! readers "$changes" "$paths" "$script" "$workspace" \
    && crates=$(python3 -I "$script" --root "$workspace" <<< "$paths") \
    && libraries=$(python3 -I "$script" --root "$workspace" --library <<< "$paths") \
    || crates=""
  if [ -n "$read_by" ]; then
    return
  fi
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
