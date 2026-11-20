#!/usr/bin/env bash
# The same two probes as run_below_1e6.sh (point (1, 54), eps = 1e-8 and 1e-10, L_w = 1024), with p_max = 8 and 16
# instead of 4, because the cost at tight eps depends on p_max and the GG-1 paper does not state it.
# usage: run_below_1e6_pmax.sh <pilot binary> <out.csv> [cap_s=1800]
P=$1; O=$2; CAP=${3:-1800}
echo "pmax,$("$P" --header | head -1)" > "$O"
for pm in 8 16; do for e in 1e-8 1e-10; do
  ( echo "$pm,$(timeout $((CAP * 3)) "$P" gg1 1 54 $e 1024 1000 $CAP $pm)" >> "$O" ) &
done; done; wait
