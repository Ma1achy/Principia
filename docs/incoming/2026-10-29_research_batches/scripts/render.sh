#!/bin/sh
# render.sh <pdf> <outdir> <first> <last> [dpi]: rasterise pages to PNG (page images; no text layer used)
mkdir -p "$2"; pdftoppm -r "${5:-200}" -png -f "$3" -l "$4" "$1" "$2/p"
