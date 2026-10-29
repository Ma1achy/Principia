#!/usr/bin/env bash
# Cost scaling probes for GG-1, all at eps >= 1e-6. Four independent streams (run them side by side):
#   e3, e4, e5: the same three cheap points at eps = 1e-3, 1e-4, 1e-5, L_w = 1024
#   lw        : one point at L_w = 128/256/512/1024, eps = 1e-6
# usage: run_scaling.sh <pilot binary> <stream: e3|e4|e5|lw> <out.csv>
set -euo pipefail
P=$1; S=$2; O=$3
"$P" --header | head -1 > "$O"
case $S in
  e3|e4|e5)
    e=1e-${S#e}
    for pt in "27 24" "1 54" "14 20"; do "$P" gg1 $pt $e 1024 1000 600 >> "$O"; done ;;
  lw)
    for lw in 128 256 512 1024; do "$P" gg1 1 54 1e-6 $lw 1000 600 >> "$O"; done ;;
esac
