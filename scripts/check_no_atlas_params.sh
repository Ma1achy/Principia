#!/usr/bin/env bash
# Fails if the Three Body Orbits operator's atlas-params.json, or any copy of it, is tracked.
# It was sent by email and not published, so it stays out of the repo (docs/incoming/2026-10-26_orbit_catalogues/,
# "Data terms"); .gitignore keeps it out of `git add`, and this catches `git add -f`, a rename, a gzip or zip copy, and
# a re-serialisation that keeps each key and its colon on one line.
# Three tests:
# - the file names;
# - the exact file's sha256, on tracked .json and .txt files and on the decompressed content of tracked .gz files and
#   of each member of tracked .zip files;
# - its "ic_hp"/"period_hp" keys, which no other file in the repo uses, quoted with " or ' and followed by a colon on
#   the same line (JSON, minified or not, CRLF, a Python repr), in every tracked file and in the same decompressed
#   content.
# Limits, not caught: a key split from its colon across lines, YAML (unquoted keys), UTF-16 or another encoding that
# is not ASCII-compatible, an archive nested in an archive, a compression format other than gzip and zip, and a .zip
# member whose name contains a newline.
# Fails closed: a git, grep, unzip or sha256sum failure exits 2, and an archive that does not decompress exits 1;
# neither prints "ok".
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
fail=0
sha=a5378e2b1c099a02bce36f85b12eec6449623fdf4ebd18bbed15de0135413296
keyre="[\"'](ic_hp|period_hp)[\"'][[:space:]]*:"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
broken() { echo "check_no_atlas_params: $1 failed (exit $2); the check is incomplete" >&2; exit 2; }

# 1. Names. The guard's own two files carry the name; they are excluded by exact path, so a copy under any other name
# is caught.
rc=0; files=$(git ls-files -- ':!:scripts/check_no_atlas_params.sh' ':!:.github/workflows/no-atlas-params.yml') || rc=$?
[ "$rc" = 0 ] || broken "git ls-files" "$rc"
rc=0; names=$(grep -i -E 'atlas[-_]?params' <<<"$files") || rc=$?
[ "$rc" -le 1 ] || broken grep "$rc"
if [ -n "$names" ]; then echo "atlas-params tracked by name:"; echo "$names"; fail=1; fi

# 2. The sha256 of plain files.
rc=0; git ls-files -z -- '*.json' '*.txt' ':!:*.lock' >"$tmp/plain" || rc=$?
[ "$rc" = 0 ] || broken "git ls-files" "$rc"
while IFS= read -r -d '' f; do
  if [ -f "$f" ]; then
    rc=0; h=$(sha256sum <"$f") || rc=$?
    [ "$rc" = 0 ] || broken sha256sum "$rc"
    if [ "${h%% *}" = "$sha" ]; then echo "atlas-params tracked as $f"; fail=1; fi
  fi
done <"$tmp/plain"

# 3. Keys in every tracked file.
rc=0; keys=$(git grep -l -E "$keyre" -- ':!:scripts/check_no_atlas_params.sh') || rc=$?
[ "$rc" -le 1 ] || broken "git grep" "$rc"
if [ -n "$keys" ]; then echo "atlas-params content (its \"ic_hp\"/\"period_hp\" keys) tracked in:"; echo "$keys"; fail=1; fi

# 2 and 3 on decompressed content, in the file $1; $2 names it in the output.
check_content() {
  local h rc=0
  h=$(sha256sum <"$1") || rc=$?
  [ "$rc" = 0 ] || broken sha256sum "$rc"
  if [ "${h%% *}" = "$sha" ]; then echo "atlas-params tracked as $2"; fail=1; fi
  grep -a -q -E "$keyre" "$1" || rc=$?
  [ "$rc" -le 1 ] || broken grep "$rc"
  if [ "$rc" = 0 ]; then echo "atlas-params content (its \"ic_hp\"/\"period_hp\" keys) tracked in $2"; fail=1; fi
}

# Compressed files, decompressed. One that does not decompress fails the check.
rc=0; git ls-files -z -- '*.gz' >"$tmp/gz" || rc=$?
[ "$rc" = 0 ] || broken "git ls-files" "$rc"
while IFS= read -r -d '' f; do
  [ -f "$f" ] || continue
  if ! gzip -dc -- "$f" >"$tmp/content"; then echo "cannot decompress $f, so it cannot be checked"; fail=1; continue; fi
  check_content "$tmp/content" "$f"
done <"$tmp/gz"
rc=0; git ls-files -z -- '*.zip' >"$tmp/zip" || rc=$?
[ "$rc" = 0 ] || broken "git ls-files" "$rc"
while IFS= read -r -d '' f; do
  [ -f "$f" ] || continue
  if ! members=$(unzip -Z1 -- "$f" </dev/null); then echo "cannot list $f, so it cannot be checked"; fail=1; continue; fi
  while IFS= read -r m; do
    case "$m" in '' | */) continue ;; esac
    # unzip reads a member argument as a wildcard pattern; bracket its wildcard characters so it names $m exactly.
    rc=0; pat=$(sed -e 's/\[/[[]/g' -e 's/\*/[*]/g' -e 's/?/[?]/g' <<<"$m") || rc=$?
    [ "$rc" = 0 ] || broken sed "$rc"
    if ! unzip -p -- "$f" "$pat" </dev/null >"$tmp/content"; then
      echo "cannot decompress $m in $f, so it cannot be checked"; fail=1; continue
    fi
    check_content "$tmp/content" "$f:$m"
  done <<<"$members"
done <"$tmp/zip"

[ "$fail" = 0 ] && echo "ok: atlas-params.json is not tracked"
exit "$fail"
