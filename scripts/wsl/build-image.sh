#!/usr/bin/env bash
# Builds the WSL2 distro image `principia-wsl` (RQ-751 option (c); the owner's words of 9 Nov 2026: "the distro image
# pinned and integrity-checked like the runner tree, rebuilt from a script, no Docker"). It runs on Linux with
# passwordless sudo, for its chroot: `win-gpu.yml`'s hosted `wsl-image` job, or a Linux machine by hand. It compiles
# only the two cargo tools, inside the chroot.
#
#   scripts/wsl/build-image.sh <out-dir>
#
# writes <out-dir>/rootfs.tar.gz, the image `wsl --import` takes; <out-dir>/rootfs.sha256, its bytes' SHA-256, for the
# copy's integrity; and <out-dir>/manifest.txt, `manifest.sh`'s description of what is in it. The manifest's SHA-256 is
# the image's pinned value, `scripts/wsl/manifest.sha256`: the tarball's bytes vary from build to build (times, logs),
# what the manifest lists doesn't. The script prints the manifest's hash and the pinned one, and fails if a pin is set
# and differs (proof 5).
#
# The image: Ubuntu 24.04's WSL rootfs, pinned by URL, version and SHA-256 and verified before use; every package from
# snapshot.ubuntu.com at one pinned date, upgraded to it; sudo purged; one ordinary user, `ci` (uid 1000), with no sudo
# and no sudoers entry; `/etc/wsl.conf` exactly `scripts/wsl/wsl.conf` (automount and interop off, no Windows PATH,
# default user ci; no boot section, so no systemd); no first-run setup (`/etc/wsl-distribution.conf` removed);
# rustup installed as ci, pinned to `rust-toolchain.toml`'s channel and components; cargo-nextest and cargo-mutants at
# the versions `mutants.yml` installs; and ci's cargo config holding what `.github/actions/cargo-ci-config` gives
# CI's jobs, so builds inside match hosted CI's.
set -euo pipefail

BASE_URL=https://releases.ubuntu.com/noble/ubuntu-24.04.5-wsl-amd64.wsl
BASE_VERSION=24.04.5
BASE_SHA256=bb415d824822c4b878125729af451a5d18fb13d1cf5cbed9a7393ad64ac6039e
SNAPSHOT=20261108T000000Z
RUSTUP_VERSION=1.29.3
RUSTUP_SHA256=b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55
NEXTEST_VERSION=0.9.146
MUTANTS_VERSION=27.1.0
# What builds and tests the workspace on the CPU: a C toolchain and pkg-config for the crates' build scripts, python3
# with PyYAML for xtask's tests, git and curl, and util-linux and procps for `scripts/contain.sh`.
PACKAGES=(build-essential pkg-config python3 python3-yaml git curl ca-certificates util-linux procps)

out=${1:?usage: build-image.sh <out-dir>}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd "$here/../.." && pwd)
mkdir -p "$out"
out=$(cd "$out" && pwd)
work=$(mktemp -d)
root="$work/rootfs"

cleanup() {
  for m in dev/pts dev proc sys; do
    sudo umount "$root/$m" 2> /dev/null || true
  done
  sudo rm -rf "$work"
}
trap cleanup EXIT

in_root() { sudo chroot "$root" env DEBIAN_FRONTEND=noninteractive LC_ALL=C "$@"; }
as_ci() { sudo chroot --userspec=1000:1000 "$root" env -i HOME=/home/ci USER=ci PATH=/home/ci/.cargo/bin:/usr/bin:/bin "$@"; }

echo "base: Ubuntu $BASE_VERSION, $BASE_URL"
curl -fsSL --retry 3 -o "$work/base.tar.gz" "$BASE_URL"
echo "$BASE_SHA256  $work/base.tar.gz" | sha256sum -c -
sudo mkdir "$root"
sudo tar -xzpf "$work/base.tar.gz" -C "$root" --numeric-owner

for m in dev dev/pts proc sys; do
  sudo mount --bind "/$m" "$root/$m"
done
# The build's own name resolution, for apt and cargo; the image's own resolv.conf, a file or a link, goes back after.
sudo mv "$root/etc/resolv.conf" "$work/resolv.conf.image"
sudo cp /etc/resolv.conf "$root/etc/resolv.conf"
# No service starts in the chroot.
printf '#!/bin/sh\nexit 101\n' | sudo tee "$root/usr/sbin/policy-rc.d" > /dev/null
sudo chmod 755 "$root/usr/sbin/policy-rc.d"

# Every package from one dated snapshot, nothing from the live archive.
sudo rm -f "$root/etc/apt/sources.list"
sudo tee "$root/etc/apt/sources.list.d/ubuntu.sources" > /dev/null << EOF
Types: deb
URIs: https://snapshot.ubuntu.com/ubuntu/$SNAPSHOT
Suites: noble noble-updates noble-security
Components: main universe
Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg
EOF
in_root apt-get -q update
in_root apt-get -q -y -o Dpkg::Options::=--force-confold dist-upgrade
in_root apt-get -q -y --no-install-recommends install "${PACKAGES[@]}"
in_root apt-get -q -y purge sudo
in_root apt-get -q -y clean
sudo rm -rf "$root/var/lib/apt/lists/"*

# One ordinary user, with no sudo and no sudoers entry; its password locked.
in_root useradd --create-home --uid 1000 --user-group --shell /bin/bash ci
in_root passwd -q -l ci
sudo rm -rf "$root/etc/sudoers.d"

sudo install -m 0644 -o 0 -g 0 "$here/wsl.conf" "$root/etc/wsl.conf"
sudo rm -f "$root/etc/wsl-distribution.conf"
sudo install -d -m 0755 "$root/usr/local/share/principia-wsl"
echo "$BASE_SHA256" | sudo tee "$root/usr/local/share/principia-wsl/base.sha256" > /dev/null
echo "$SNAPSHOT" | sudo tee "$root/usr/local/share/principia-wsl/snapshot" > /dev/null

# rustup as ci, pinned to rust-toolchain.toml.
read -r channel components < <(python3 - "$repo/rust-toolchain.toml" << 'EOF'
import sys, tomllib
t = tomllib.load(open(sys.argv[1], "rb"))["toolchain"]
print(t["channel"], ",".join(t["components"]))
EOF
)
echo "toolchain: $channel, components $components"
curl -fsSL --retry 3 -o "$work/rustup-init" \
  "https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/x86_64-unknown-linux-gnu/rustup-init"
echo "$RUSTUP_SHA256  $work/rustup-init" | sha256sum -c -
sudo install -m 0755 "$work/rustup-init" "$root/tmp/rustup-init"
as_ci /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain "$channel" --component "$components"
sudo rm -f "$root/tmp/rustup-init"
as_ci cargo install --locked "cargo-nextest@$NEXTEST_VERSION" "cargo-mutants@$MUTANTS_VERSION"
sudo rm -rf "$root/home/ci/.cargo/registry" "$root/home/ci/.cargo/git"
# The same bytes `.github/actions/cargo-ci-config/config.sh` appends to a hosted job's fresh CARGO_HOME.
as_ci sh -c 'printf '\''[profile.dev.package."*"]\ndebug = false\n[profile.test.package."*"]\ndebug = false\n'\'' > /home/ci/.cargo/config.toml'

# shellcheck disable=SC2024 # the script and the manifest are this user's files, read and written as this user
sudo chroot "$root" /bin/sh -s < "$here/manifest.sh" > "$out/manifest.txt"

sudo rm -f "$root/usr/sbin/policy-rc.d"
sudo mv -f "$work/resolv.conf.image" "$root/etc/resolv.conf"
for m in dev/pts dev proc sys; do
  sudo umount "$root/$m"
done
sudo find "$root/tmp" "$root/var/tmp" -mindepth 1 -delete
sudo tar --numeric-owner -czf "$out/rootfs.tar.gz" -C "$root" .
(cd "$out" && sha256sum rootfs.tar.gz | cut -d' ' -f1 > rootfs.sha256)

hash=$(sha256sum < "$out/manifest.txt" | cut -d' ' -f1)
pinned=$(tr -d '[:space:]' < "$here/manifest.sha256")
echo "manifest sha256: $hash"
echo "pinned:          $pinned"
case "$pinned" in
  "$hash") echo "proof 5: the rebuild reproduces the pinned manifest" ;;
  *[!0-9a-f]* | "") echo "no manifest hash is pinned yet: pin $hash in scripts/wsl/manifest.sha256 by a reviewed PR" ;;
  *)
    echo "::error::build-image.sh: the manifest's sha256 $hash is not the pinned $pinned"
    exit 1
    ;;
esac
