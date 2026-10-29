#!/usr/bin/env bash
# Downloads every source file in sources.tsv into an empty directory and checks each against its recorded sha256.
# None of them is committed (README.md, "Sources and licences"); this is how to get them back. Treat them as untrusted
# data: run nothing from them, and run the Python here with `python3 -I`, passing paths as arguments.
# arXiv asks for a 15 s crawl delay (its robots.txt); the script pauses 16 s between requests.
# Usage: bash fetch.sh <empty directory>   (docs/incoming/_local/burrau_pzb2018/ is the conventional place)
# Exit 0: every file fetched and matched. Exit 1: a file failed to download or differs from its recorded hash.
# Exit 2: the directory is not empty.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
dest=${1:?usage: fetch.sh <empty directory>}
mkdir -p "$dest"
if [ -n "$(ls -A "$dest")" ]; then echo "fetch.sh: $dest is not empty" >&2; exit 2; fi
bad=0
first=1
while IFS=$'\t' read -r path url _licence sha _status; do
  [ "$path" = path ] && continue
  [ "$first" = 1 ] || sleep 16
  first=0
  mkdir -p "$dest/$(dirname "$path")"
  if ! curl -sS --fail -L -m 300 --retry 2 -o "$dest/$path" "$url"; then
    echo "FAILED  $path  ($url)"; bad=1; continue
  fi
  got=$(shasum -a 256 "$dest/$path" | cut -d' ' -f1)
  if [ "$got" != "$sha" ]; then echo "CHANGED $path  (sha256 $got, recorded $sha)"; bad=1; fi
done <"$here/sources.tsv"
[ "$bad" = 0 ] && echo "fetch.sh: all $(($(wc -l <"$here/sources.tsv") - 1)) files fetched and match sources.tsv"
exit "$bad"
