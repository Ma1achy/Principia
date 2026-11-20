#!/bin/sh
# ocr_cells.sh <page.png> <y_first> <y_last> <n_rows> <half_height> <half_width> <x_centre>...
# OCR each table cell on its own (used where whole-table OCR flips glyphs): crops a box around each
# (x_centre, row centre), pads it white, and runs the Vision helper ($OCR_BIN). Prints one CSV row per table row.
set -e
img=$1; y0=$2; y1=$3; n=$4; hh=$5; hw=$6; shift 6
t=$(mktemp -d)
i=0
while [ $i -lt $n ]; do
  y=$(python3 -I -c "print(round($y0 + ($y1 - $y0) * $i / ($n - 1)))")
  line=""
  for x in "$@"; do
    magick "$img" -crop $((2*hw))x$((2*hh))+$((x-hw))+$((y-hh)) +repage -bordercolor white -border 40 "$t/c.png"
    v=$("$OCR_BIN" "$t/c.png" | cut -f2 | tr '\n' ' ' | sed 's/ *$//')
    line="$line,$v"
  done
  echo "${line#,}"; i=$((i+1))
done
rm -rf "$t"
