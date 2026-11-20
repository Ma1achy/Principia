#!/usr/bin/env bash
# Fetches Horn et al. 2024's 3-body files from Zenodo on demand (CC BY 4.0, doi:10.5281/zenodo.11032352) and checks
# Zenodo's md5. Never commit them. Credit the authors. They are pandas stores whose object columns unpickle: treat them
# as untrusted data, read block0_values as plain arrays, and never pandas.read_hdf them.
# Usage: bash fetch.sh <directory> [all_runs|training]; the default is training (9.7 MB); all_runs is 930 MB.
set -euo pipefail
dest=${1:?usage: fetch.sh <directory> [all_runs|training]}; which=${2:-training}
case "$which" in
  all_runs) f=circ_3body_all_runs.h5.1; want=873fac461d67f57db5d4d389bab3bed0 ;;
  training) f=circ_3body_training.h5.1; want=e8416577e763f349eb18c314450c07af ;;
  *) echo "unknown: $which"; exit 2 ;;
esac
mkdir -p "$dest"
[ -e "$dest/$f" ] || curl -sS --fail -A "principia-fetch/1.0" -o "$dest/$f" "https://zenodo.org/api/records/11032352/files/$f/content"
got=$(if command -v md5 >/dev/null; then md5 -q "$dest/$f"; else md5sum "$dest/$f" | cut -d' ' -f1; fi)
[ "$got" = "$want" ] && echo "OK $f" || { echo "CHANGED $f: $got, wanted $want"; exit 1; }
