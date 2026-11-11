#!/usr/bin/env bash
# The steps of `.github/actions/mesa-debs` (its `action.yml` says what each is for):
#   mesa_debs.sh key      the cache key, `key=…` to $GITHUB_OUTPUT, from PACKAGES and the runner's RUNNER_OS, ImageOS
#                         and ImageVersion; none when any is unset, so a runner that is not GitHub-hosted caches nothing
#   mesa_debs.sh install  on a hit (HIT=true), the cached packages installed with dpkg, offline, and each of PACKAGES
#                         checked installed: `installed=true`; otherwise, or when they do not install, `installed=false`,
#                         and the apt step after installs as before. It notes the packages already downloaded, which
#                         `collect` leaves out.
#   mesa_debs.sh collect  the packages the apt step downloaded, copied to the cached folder: `count=<n>`.
# The folders: MESA_DEBS_DIR (the cached one, ~/.cache/mesa-debs), MESA_APT_ARCHIVES (apt's download folder,
# /var/cache/apt/archives) and MESA_DEBS_BEFORE (the note, under RUNNER_TEMP). It never fails its step: a cache that
# does not serve leaves the install to apt.
set -u
dir="${MESA_DEBS_DIR:-$HOME/.cache/mesa-debs}"
archives="${MESA_APT_ARCHIVES:-/var/cache/apt/archives}"
before="${MESA_DEBS_BEFORE:-${RUNNER_TEMP:-/tmp}/mesa-debs-before}"
out="${GITHUB_OUTPUT:-/dev/null}"

case "${1:-}" in
  key)
    if [ -n "${PACKAGES:-}" ] && [ -n "${RUNNER_OS:-}" ] && [ -n "${ImageOS:-}" ] && [ -n "${ImageVersion:-}" ]; then
      # shellcheck disable=SC2086 # the list's words, as the apt step names them
      list=$(printf '%s+' $PACKAGES)
      echo "key=mesa-gpu-lavapipe-smoke-$RUNNER_OS-$ImageOS-$ImageVersion-${list%+}" >> "$out"
    else
      echo "mesa-debs: no GitHub-hosted image named (ImageOS, ImageVersion): nothing cached"
    fi
    ;;
  install)
    ls "$archives" 2> /dev/null | grep '\.deb$' > "$before" || true
    installed=false
    if [ "${HIT:-}" = true ] && ls "$dir"/*.deb > /dev/null 2>&1; then
      if sudo timeout -k 10 300 dpkg -i "$dir"/*.deb; then
        installed=true
        for p in ${PACKAGES:-}; do
          if [ "$(dpkg-query -W -f='${Status}' "$p" 2> /dev/null)" != "install ok installed" ]; then
            echo "mesa-debs: $p is not installed after the cached packages"
            installed=false
          fi
        done
      fi
      [ "$installed" = true ] || echo "::warning::the cached Mesa packages did not install; the apt step installs them"
    fi
    echo "installed=$installed" >> "$out"
    echo "mesa-debs: installed from the cache: $installed"
    ;;
  collect)
    mkdir -p "$dir"
    n=0
    for f in "$archives"/*.deb; do
      [ -e "$f" ] || continue
      base=$(basename "$f")
      grep -qxF "$base" "$before" 2> /dev/null && continue
      cp "$f" "$dir/" && n=$((n + 1))
    done
    echo "count=$n" >> "$out"
    [ "$n" -gt 0 ] || echo "mesa-debs: nothing downloaded to $archives, nothing to keep"
    ;;
  *)
    echo "usage: mesa_debs.sh key|install|collect" >&2
    exit 2
    ;;
esac
exit 0
