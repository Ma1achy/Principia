#!/usr/bin/env bash
# The two probes below eps = 1e-6 that the human approved on 29 Oct 2026: ONE point each, eps = 1e-8 and 1e-10,
# L_w = 1024, p_max = 4, T capped at 1000, reversibility test included. The same point (1, 54) is used for both.
# usage: run_below_1e6.sh <pilot binary> <eps> <out.csv> [cap_s=3600]
set -euo pipefail
P=$1; E=$2; O=$3; CAP=${4:-3600}
"$P" --header | head -1 > "$O"
timeout $((CAP * 3)) "$P" gg1 1 54 "$E" 1024 1000 "$CAP" 4 >> "$O"
