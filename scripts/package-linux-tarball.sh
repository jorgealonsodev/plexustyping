#!/usr/bin/env bash
# Builds the portable Linux release archive:
#   <outdir>/plexustyping-<version>-linux-x86_64.tar.gz
# with a single top directory plexustyping-<version>/ holding the launcher
# (plexustyping), the real binary (plexustyping-bin), plexustyping.desktop,
# the hicolor icons, the licences, a Spanish README and install.sh.
# Corpora, layouts and themes are compiled into the binary, so no separate
# resources are shipped.
#
# Usage: scripts/package-linux-tarball.sh <binary> <version> <outdir>
# Prints the path of the archive it wrote.
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Uso: package-linux-tarball.sh <binario> <versión> <directorio-de-salida>" >&2
  exit 2
fi

binary=$1
version=$2
outdir=${3%/}
[[ -n "$outdir" ]] || outdir=/

if [[ ! -f "$binary" ]]; then
  echo "Error: no existe el binario: $binary" >&2
  exit 1
fi
if [[ ! "$version" =~ ^[0-9A-Za-z][0-9A-Za-z.+~-]*$ ]]; then
  echo "Error: versión no válida: $version" >&2
  exit 1
fi

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
linux="$repo/scripts/linux"
icons="$repo/src-tauri/icons"
top="plexustyping-$version"
name="$top-linux-x86_64.tar.gz"

staging=$(mktemp -d)
cleanup() { rm -rf "$staging"; }
trap cleanup EXIT
pkg="$staging/$top"
mkdir -p "$pkg"

install -m 755 "$linux/plexustyping" "$pkg/plexustyping"
install -m 755 "$binary" "$pkg/plexustyping-bin"
install -m 755 "$linux/install.sh" "$pkg/install.sh"
install -m 644 "$linux/plexustyping.desktop" "$pkg/plexustyping.desktop"
install -m 644 "$linux/README" "$pkg/README"
for f in LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.md; do
  install -m 644 "$repo/$f" "$pkg/$f"
done

# hicolor size -> source icon in src-tauri/icons
for pair in 32x32:32x32.png 64x64:64x64.png 128x128:128x128.png \
  256x256:128x128@2x.png 512x512:icon.png; do
  size=${pair%%:*}
  mkdir -p "$pkg/icons/hicolor/$size/apps"
  install -m 644 "$icons/${pair#*:}" "$pkg/icons/hicolor/$size/apps/plexustyping.png"
done

mkdir -p "$outdir"
tar --sort=name --owner=0 --group=0 --numeric-owner \
  -C "$staging" -czf "$outdir/$name" "$top"
echo "$outdir/$name"
