#!/usr/bin/env bash
# p_max sensitivity at eps = 1e-6, L_w = 1024 on two cheap points (p_max is not stated in the GG-1 paper).
# Output rows are prefixed with pmax. usage: run_pmax_probe.sh <pilot binary> <out.csv>
P=$1; O=$2
echo "pmax,$("$P" --header | head -1)" > "$O"
for pm in 2 4 8 16; do for pt in "1 54" "14 20"; do
  ( echo "$pm,$("$P" gg1 $pt 1e-6 1024 1000 300 $pm)" >> "$O" ) &
done; done; wait
