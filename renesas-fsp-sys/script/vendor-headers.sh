#!/usr/bin/env bash
#
# Re-vendor the FSP interface headers in `renesas-fsp-sys/vendor/` from a
# Renesas FSP pack distribution. Only the interface layer (`ra/fsp/inc/api/`)
# is vendored, and only the subset in HEADERS below -- see README.md for why
# the instance layer cannot be included.
#
# Usage:
#   script/vendor-headers.sh path/to/FSP_Packs_vX.Y.Z.zip
#
# The pack distribution is a few hundred MB, so it is not checked in. Download
# it from the Renesas FSP releases page --
# https://github.com/renesas/fsp/releases -- or copy it out of an e2 studio
# installation under `internal/projectgen/ra/packs/`. Requires `unzip`.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_root="$(dirname "$here")"
repo_root="$(dirname "$crate_root")"

if [[ $# -ne 1 ]]; then
  echo "usage: ${BASH_SOURCE[0]} path/to/FSP_Packs_vX.Y.Z.zip" >&2
  exit 2
fi
packs_zip="$1"

# The subset of the interface layer this crate binds. Adding a peripheral takes
# three further edits beyond this list; README.md has the sequence.
HEADERS=(
  fsp_common_api.h
  r_ioport_api.h
  r_timer_api.h
)

if [[ ! -f "$packs_zip" ]]; then
  echo "error: pack distribution not found: $packs_zip" >&2
  exit 1
fi

# The distribution is a zip of packs; each pack is itself a zip.
work="$(mktemp -d "${TMPDIR:-/tmp}/fsp-vendor.XXXXXX")"
trap 'rm -rf "$work"' EXIT

ra_pack_path="$(unzip -Z1 "$packs_zip" | grep -E '/Renesas\.RA\.[0-9.]+\.pack$' | head -1)"
if [[ -z "$ra_pack_path" ]]; then
  echo "error: no Renesas.RA.<version>.pack inside $packs_zip" >&2
  exit 1
fi

fsp_version="$(basename "$ra_pack_path" | sed -E 's/^Renesas\.RA\.(.*)\.pack$/\1/')"
echo "Renesas.RA $fsp_version"

unzip -q -o "$packs_zip" "$ra_pack_path" -d "$work"
unzip -q -o "$work/$ra_pack_path" 'ra/fsp/inc/*' -d "$work/pack"

src="$work/pack/ra/fsp/inc"
dst="$crate_root/vendor/fsp/inc"

# Replace rather than merge, so a header dropped from HEADERS actually leaves.
# Guarded because $dst is interpolated into an `rm -rf`.
if [[ "$dst" != "$crate_root/vendor/"* ]]; then
  echo "error: refusing to remove unexpected path: $dst" >&2
  exit 1
fi
rm -rf "$dst"
mkdir -p "$dst/api"

cp "$src/fsp_version.h" "$dst/fsp_version.h"
for h in "${HEADERS[@]}"; do
  cp "$src/api/$h" "$dst/api/$h"
done

printf '%s\n' "$fsp_version" > "$crate_root/vendor/FSP_VERSION"

echo "vendored ${#HEADERS[@]} interface headers + fsp_version.h into ${dst#"$repo_root"/}"
echo
echo "Next: RENESAS_FSP_SYS_UPDATE_SNAPSHOT=1 cargo build -p renesas-fsp-sys, then review the diff."
