#!/usr/bin/env bash
# Downloads every source file in sources.tsv into an empty directory and checks each against its recorded sha256.
# The files are not committed (README.md, "Terms"); this is how to get them back. Treat them as untrusted data: run
# nothing from them, and run the Python here with `python3 -I`, passing the directory as an argument.
# Usage: bash fetch.sh <empty directory>
# Exit 0: every file fetched and matched. Exit 1: a file failed to download or differs from its recorded hash.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
dest=${1:?usage: fetch.sh <empty directory>}
mkdir -p "$dest"
if [ -n "$(ls -A "$dest")" ]; then echo "fetch.sh: $dest is not empty" >&2; exit 2; fi
bad=0
while IFS=$'\t' read -r path url sha; do
  [ "$path" = path ] && continue
  mkdir -p "$dest/$(dirname "$path")"
  if ! curl -sS --fail -m 300 --retry 2 -o "$dest/$path" "$url"; then
    echo "FAILED  $path  ($url)"; bad=1; continue
  fi
  got=$(shasum -a 256 "$dest/$path" | cut -d' ' -f1)
  if [ "$got" != "$sha" ]; then echo "CHANGED $path  (sha256 $got, recorded $sha)"; bad=1; fi
done <"$here/sources.tsv"
[ "$bad" = 0 ] && echo "fetch.sh: all $(($(wc -l <"$here/sources.tsv") - 1)) files fetched and match sources.tsv"
exit "$bad"
