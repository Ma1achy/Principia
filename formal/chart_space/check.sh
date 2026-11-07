#!/usr/bin/env bash
# Build the library, then fail on any escape hatch in the source or any axiom beyond Lean's three.
set -euo pipefail
cd "$(dirname "$0")"

hatches='\b(sorry|admit|native_decide|sorryAx)\b|^[[:space:]]*((private|protected|noncomputable|unsafe|partial)[[:space:]]+)*(axiom|opaque)\b|\bimplemented_by\b|\bextern\b'
if grep --recursive --line-number --extended-regexp "$hatches" --include='*.lean' ChartSpace ChartSpace.lean; then
  echo "check: an escape hatch is in the source" >&2
  exit 1
fi

lake build ChartSpace

# Axioms.lean's `#check_axioms` fails on any axiom beyond Lean's three.
lake env lean ChartSpace/Axioms.lean
echo "check: $(grep --count '^#check_axioms ' ChartSpace/Axioms.lean) theorems, only propext, Classical.choice, Quot.sound"
