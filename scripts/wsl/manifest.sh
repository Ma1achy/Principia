#!/bin/sh
# The WSL2 image's manifest, run inside the system it describes: by `build-image.sh` in the image's chroot, and by
# `.github/scripts/wsl.ps1` in the registered distro, as `ci`, which re-derives it from what is really there. The
# manifest's SHA-256 is the image's pinned value, `scripts/wsl/manifest.sha256` (proof 5: a rebuild reproduces it).
#
# Every line is deterministic for one build's inputs: the base tarball's SHA-256 and the snapshot date the build
# recorded, the SHA-256 of /etc/wsl.conf's bytes and of ci's cargo config, the sorted `package=version` lines from
# dpkg-query, the rustup toolchain with its installed components, rustc's and cargo's versions, the two cargo tools,
# and the account: ci's passwd line, its groups, and whether sudo exists. No time, host or path of the build is in it.
set -eu
export LC_ALL=C
lib=/usr/local/share/principia-wsl
home=/home/ci
export RUSTUP_HOME="$home/.rustup" CARGO_HOME="$home/.cargo"
bin="$CARGO_HOME/bin"
# Out of any directory whose rust-toolchain.toml would override the default.
cd /

echo "principia-wsl manifest 1"
echo "base-sha256 $(cat "$lib/base.sha256")"
echo "snapshot $(cat "$lib/snapshot")"
echo "wsl.conf-sha256 $(sha256sum < /etc/wsl.conf | cut -d' ' -f1)"
echo "cargo-config-sha256 $(sha256sum < "$CARGO_HOME/config.toml" | cut -d' ' -f1)"
echo "account $(getent passwd ci)"
echo "groups $(id -nG ci)"
if command -v sudo > /dev/null 2>&1 || [ -e /etc/sudoers ]; then echo "sudo present"; else echo "sudo absent"; fi
echo "toolchains"
"$bin/rustup" toolchain list
toolchain=$("$bin/rustup" toolchain list | sed -n 's/ (.*//; 1p')
echo "components $toolchain"
"$bin/rustup" component list --installed --toolchain "$toolchain"
echo "rustc $("$bin/rustc" "+$toolchain" -V)"
echo "cargo $("$bin/cargo" "+$toolchain" -V)"
echo "cargo-nextest $("$bin/cargo-nextest" nextest --version | head -1)"
echo "cargo-mutants $("$bin/cargo-mutants" mutants --version | head -1)"
echo "packages"
dpkg-query -W -f '${Package}=${Version}\n' | sort
