#!/usr/bin/env bash
# Build Brutus as pinned (pgbreen/NvMgeneration @ 98906f4, GPL-3.0) and our driver src/pilot.cpp against it.
# Usage: build.sh <work-dir>      (the pinned source is cloned into <work-dir>/nvmgeneration; nothing is vendored here)
set -euo pipefail
PIN=98906f472f66b127bde62f771410b4974d116a3b
mkdir -p "$1"
W=$(cd "$1" && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
if [ ! -d "$W/nvmgeneration/.git" ]; then
  GIT_LFS_SKIP_SMUDGE=1 git clone https://github.com/pgbreen/nvmgeneration "$W/nvmgeneration"
fi
cd "$W/nvmgeneration"
[ "$(git rev-parse HEAD)" = "$PIN" ] || git checkout --detach "$PIN"
[ "$(git rev-parse HEAD)" = "$PIN" ] || { echo "not at the pinned commit"; exit 1; }
[ -z "$(git status --porcelain --untracked-files=no)" ] || { echo "pinned tree is modified"; exit 1; }
make clean >/dev/null 2>&1 || true
# Deviations from the makefile as shipped, all passed through CXXFLAGS (no source edit):
#   -DMPFR_USE_NO_MACRO     the bundled mpreal.h 3.6.2 does not compile against MPFR 4.2's function-like macros
#   -Wl,--no-as-needed      the makefile puts -lmpfr -lgmp before the objects on the link line; Ubuntu's linker drops them
export CXXFLAGS="-DMPFR_USE_NO_MACRO -Wl,--no-as-needed -w"
make main.exe > "$W/build_stock.log" 2>&1       # the stock executable, unmodified source
sha256sum main.exe > "$W/main.exe.sha256"
# Our driver: same compiler flags as the makefile's CFLAGS (-O1 -std=gnu++11), linked to the pinned objects.
g++ -O1 -std=gnu++11 -DMPFR_USE_NO_MACRO -Wl,--no-as-needed -w -I. -o "$W/pilot" "$HERE/src/pilot.cpp" \
    Brutus.o Bulirsch_Stoer.o Cluster.o Star.o -lmpfr -lgmp
sha256sum "$W/pilot" > "$W/pilot.sha256"
echo "built $W/pilot and $W/nvmgeneration/main.exe at $(git rev-parse HEAD)"
