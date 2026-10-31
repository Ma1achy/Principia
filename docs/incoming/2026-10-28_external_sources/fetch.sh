#!/usr/bin/env bash
# Re-fetch the publicly fetchable files of the 2026-10-28 external-sources ingest into the gitignored
# docs/incoming/_local/ tree, and check each against sources.tsv's sha256.
#
# Run from the repository root:  bash docs/incoming/2026-10-28_external_sources/fetch.sh
#
# Politeness (checked 29 Oct 2026): one request at a time; 16 s between requests (arxiv.org/robots.txt asks for a
# crawl delay of 15 s, zenodo.org's 10 s; the same pause is used everywhere). arxiv.org allows /pdf; zenodo.org allows
# /records/<id>/files; www.scl.rs and disser.spbu.ru disallow none of these paths; media.springernature.com allows
# all; ww2.ii.uj.edu.pl/robots.txt redirects to the faculty home page (no rules). link.springer.com's robots.txt
# disallows AI agents, so nothing is fetched from it (Dandekar & Kol 2025 is committed instead). The User-Agent
# names the tool only; it carries no personal address.
#
# Fetched, like every row marked 'yes': Kapela & Simo's code archive choreo.zip (GPL v2) goes to _local/ with the
# rest; GPL v2 is not a licence the owner allows for commits, so it is never committed.
#
# Not fetched (rows marked 'no' in sources.tsv): the owner's own copies (Tanikawa, Saito & Mikkola 2019 accepted
# manuscript; Orlov, Titov & Shombina 2016; Mel'nikov, Orlov & Shevchenko 2013; the Springer batch) and the
# publisher-hosted Johnstone & Rucinski 1991. They are not publicly fetchable; ask the owner.
#
# arXiv may re-render a PDF, so an arXiv file can differ in bytes from the one we read while being the same version;
# a mismatch is reported, never silently accepted, and the script does not overwrite an existing file.
set -euo pipefail

UA="Principia-ingest/1.0 (record check)"
TSV="docs/incoming/2026-10-28_external_sources/sources.tsv"
PAUSE=16
CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1   # --check: verify files present, list what would be fetched, fetch nothing

[ -f "$TSV" ] || { echo "run from the repository root" >&2; exit 2; }

first=1
mismatches=0
# columns: source path size sha256 origin url fetchable licence
while IFS=$'\t' read -r source path sha url fetchable; do
  [ "$fetchable" = "yes" ] || continue
  if [ -e "$path" ]; then
    have=$(shasum -a 256 "$path" | cut -d' ' -f1)
    if [ "$have" = "$sha" ]; then echo "present, hash ok: $path"; else echo "present, HASH MISMATCH: $path"; mismatches=$((mismatches+1)); fi
    continue
  fi
  if [ "$CHECK_ONLY" = 1 ]; then echo "would fetch: $url -> $path"; continue; fi
  [ $first -eq 1 ] || sleep "$PAUSE"
  first=0
  mkdir -p "$(dirname "$path")"
  tmp="$path.part"
  echo "fetching $url"
  curl -fsSL --retry 2 --retry-delay 30 -A "$UA" -o "$tmp" "$url"
  got=$(shasum -a 256 "$tmp" | cut -d' ' -f1)
  mv "$tmp" "$path"
  if [ "$got" = "$sha" ]; then
    echo "  ok ($source)"
  else
    echo "  HASH MISMATCH ($source): expected $sha, got $got"
    mismatches=$((mismatches+1))
  fi
done < <(tail -n +2 "$TSV" | awk -F'\t' 'BEGIN{OFS="\t"}{print $1,$2,$4,$6,$7}')

echo "done; $mismatches hash mismatch(es)"
tail -n +2 "$TSV" | awk -F'\t' '$7=="yes"{print $2}' | while read -r f; do
  if [ -e "$f" ] && ! git check-ignore -q "$f"; then echo "WARNING: $f is not gitignored"; fi
done
