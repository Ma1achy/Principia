#!/usr/bin/env bash
# Fails if the Three Body Orbits operator's atlas-params.json, or any copy of it, is tracked.
# It was sent by email and not published, so it stays out of the repo (docs/incoming/2026-10-26_orbit_catalogues/,
# "Data terms"); .gitignore keeps it out of `git add`, and this catches `git add -f`, a rename and a reformat.
# Three tests: the file names, the exact file's sha256, and its JSON keys, which no other file in the repo uses.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
fail=0
names=$(git ls-files | grep -i -E 'atlas[-_]?params' || true)
if [ -n "$names" ]; then echo "atlas-params tracked by name:"; echo "$names"; fail=1; fi
sha=a5378e2b1c099a02bce36f85b12eec6449623fdf4ebd18bbed15de0135413296
while IFS= read -r -d '' f; do
  if [ -f "$f" ] && [ "$(sha256sum "$f" | cut -d' ' -f1)" = "$sha" ]; then echo "atlas-params tracked as $f"; fail=1; fi
done < <(git ls-files -z -- '*.json' '*.txt' '*.gz' '*.zip' ':!:*.lock')
keys=$(git grep -l -E '"(ic_hp|period_hp)"[[:space:]]*:' -- ':!:scripts/check_no_atlas_params.sh' || true)
if [ -n "$keys" ]; then echo "atlas-params content (its \"ic_hp\"/\"period_hp\" keys) tracked in:"; echo "$keys"; fail=1; fi
[ "$fail" = 0 ] && echo "ok: atlas-params.json is not tracked"
exit "$fail"
