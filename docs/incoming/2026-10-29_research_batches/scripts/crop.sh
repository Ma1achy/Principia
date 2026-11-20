#!/bin/sh
# crop.sh <pdf> <page> <dpi> <y0> <y1> <out.png> [x0 x1]: render one page and crop a band (fractions of the page)
set -e
t=$(mktemp -d); pdftoppm -r "$3" -png -f "$2" -l "$2" "$1" "$t/p"; f=$(ls "$t"/p*.png)
W=$(magick identify -format %w "$f"); H=$(magick identify -format %h "$f")
x0=${7:-0}; x1=${8:-1}
magick "$f" -crop "$(python3 -I -c "print(f'{int(($x1-$x0)*$W)}x{int(($5-$4)*$H)}+{int($x0*$W)}+{int($4*$H)}')")" +repage "$6"; rm -rf "$t"
