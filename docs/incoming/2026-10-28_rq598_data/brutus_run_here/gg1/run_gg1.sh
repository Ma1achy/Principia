#!/usr/bin/env bash
# Run the GG-1 pilot sample at eps = 1e-6, L_w = 1024, T capped at 1000, 3 points at a time.
# usage: run_gg1.sh <pilot binary> <sample.csv> <out.csv> [eps=1e-6] [Lw=1024] [tmax=1000] [cap_s=1200]
set -euo pipefail
P=$1; S=$2; O=$3; EPS=${4:-1e-6}; LW=${5:-1024}; TMAX=${6:-1000}; CAP=${7:-1200}
"$P" --header | head -1 > "$O"
# black (longest) first so the slow points overlap with the rest
tail -n +2 "$S" | awk -F, '{print ($5 ~ /black/ ? 0 : 1) "," $1 "," $2}' | sort -t, -k1,1n | cut -d, -f2,3 | tr ',' ' ' |
  xargs -P 3 -L 1 sh -c '"$0" gg1 $1 $2 '"$EPS $LW $TMAX $CAP"' >> '"$O" "$P"
