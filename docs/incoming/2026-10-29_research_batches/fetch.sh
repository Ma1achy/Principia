#!/usr/bin/env bash
# Gets back the papers in sources.tsv and checks each against its recorded sha256. None is committed: no paper in
# these batches carries an open licence (README.md, "Licences"). Treat every file as untrusted data: run nothing from
# it, and run the Python here with `python3 -I`, passing paths as arguments.
#   route "arxiv":      fetched from arXiv (the owner's copy is byte-identical to that arXiv version), one request
#                       at a time, 3 s apart.
#   route "publisher":  subscription only. Place your own copy at <dir>/<path> first; this script only checks it,
#                       and otherwise prints the DOI link.
#   route "owner-copy": Burrau 1913 and Schubart 1956, the owner's copies; left out here (local only).
# Usage: bash fetch.sh <directory>
# Exit 0: every arxiv file fetched and every file present matched. Exit 1: a fetch failed or a file differs.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
dest=${1:?usage: fetch.sh <directory>}
mkdir -p "$dest"
bad=0; missing=0
while IFS=$'\t' read -r path sha doi _crossref _cite _lic route url _ads _committed; do
  [ "$path" = path ] && continue
  [ "$route" = owner-copy ] && continue
  mkdir -p "$dest/$(dirname "$path")"
  if [ "$route" = arxiv ] && [ ! -e "$dest/$path" ]; then
    if ! curl -sS --fail -m 300 -A "principia-fetch/1.0" -o "$dest/$path" "$url"; then
      echo "FAILED  $path  ($url)"; bad=1; continue
    fi
    sleep 3
  fi
  if [ ! -e "$dest/$path" ]; then echo "MANUAL  $path  (https://doi.org/$doi)"; missing=$((missing + 1)); continue; fi
  got=$(shasum -a 256 "$dest/$path" | cut -d' ' -f1)
  if [ "$got" != "$sha" ]; then echo "CHANGED $path  (sha256 $got, recorded $sha)"; bad=1; else echo "OK      $path"; fi
done <"$here/sources.tsv"
echo "fetch.sh: $missing publisher file(s) to place by hand"
exit "$bad"
