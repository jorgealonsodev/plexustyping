#!/usr/bin/env bash
# Tests scripts/package-linux-tarball.sh and the files it ships: the archive
# name and contents, the launcher's WebKitGTK check, and install.sh
# (+ --uninstall) into a temporary HOME. Uses a fake binary; needs no build.
#
# Usage: scripts/test-package-linux-tarball.sh
set -euo pipefail
# The launcher's GUI fallback is tested explicitly below; elsewhere it must not open windows.
export PLEXUSTYPING_NO_GUI=1

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
packager="$repo/scripts/package-linux-tarball.sh"

work=$(mktemp -d)
cleanup() { rm -rf "$work"; }
trap cleanup EXIT

failures=0
checks=0
pass() { checks=$((checks + 1)); }
fail() {
  checks=$((checks + 1))
  failures=$((failures + 1))
  printf 'FAIL: %s\n' "$*" >&2
}
check() { # <description> <command...>
  local d=$1; shift
  if "$@"; then pass; else fail "$d"; fi
}
expect_eq() { # <description> <expected> <actual>
  if [[ "$2" == "$3" ]]; then pass; else
    fail "$1"
    diff <(printf '%s\n' "$2") <(printf '%s\n' "$3") >&2 || true
  fi
}

version=9.8.7
top="plexustyping-$version"
archive_name="$top-linux-x86_64.tar.gz"

# Fake app binary: prints its arguments, one per line.
fakebin="$work/fake-plexustyping"
cat >"$fakebin" <<'EOF'
#!/usr/bin/env bash
echo "fake-bin"
for a in "$@"; do echo "arg:$a"; done
EOF
chmod +x "$fakebin"

# Fake ldconfig variants.
mkdir -p "$work/tools" "$work/nolibs" "$work/libs"
printf '#!/bin/sh\necho "0 libs found in cache"\n' >"$work/tools/ldconfig-empty"
cat >"$work/tools/ldconfig-webkit" <<'EOF'
#!/bin/sh
echo "1 libs found in cache"
echo "	libwebkit2gtk-4.1.so.0 (libc6,x86-64) => /usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.1.so.0"
EOF
chmod +x "$work/tools/ldconfig-empty" "$work/tools/ldconfig-webkit"
: >"$work/libs/libwebkit2gtk-4.1.so.0"

# --- packager: usage errors -------------------------------------------------
set +e
err=$("$packager" 2>&1 >/dev/null); code=$?
set -e
expect_eq "packager without arguments exits 2" 2 "$code"
expect_eq "packager usage message" \
  "Uso: package-linux-tarball.sh <binario> <versión> <directorio-de-salida>" "$err"

set +e
err=$("$packager" "$work/does-not-exist" "$version" "$work/out" 2>&1 >/dev/null); code=$?
set -e
expect_eq "packager with a missing binary exits 1" 1 "$code"
expect_eq "missing binary message" "Error: no existe el binario: $work/does-not-exist" "$err"
check "missing binary must not create the output dir" test ! -e "$work/out"

# --- packager: archive ------------------------------------------------------
out=$("$packager" "$fakebin" "$version" "$work/out")
expect_eq "packager prints the archive path" "$work/out/$archive_name" "$out"
expect_eq "output dir holds only the archive" "$archive_name" "$(ls -A "$work/out")"

expected_list="$top/
$top/LICENSE-APACHE
$top/LICENSE-MIT
$top/README
$top/THIRD_PARTY_NOTICES.md
$top/icons/
$top/icons/hicolor/
$top/icons/hicolor/128x128/
$top/icons/hicolor/128x128/apps/
$top/icons/hicolor/128x128/apps/plexustyping.png
$top/icons/hicolor/256x256/
$top/icons/hicolor/256x256/apps/
$top/icons/hicolor/256x256/apps/plexustyping.png
$top/icons/hicolor/32x32/
$top/icons/hicolor/32x32/apps/
$top/icons/hicolor/32x32/apps/plexustyping.png
$top/icons/hicolor/512x512/
$top/icons/hicolor/512x512/apps/
$top/icons/hicolor/512x512/apps/plexustyping.png
$top/icons/hicolor/64x64/
$top/icons/hicolor/64x64/apps/
$top/icons/hicolor/64x64/apps/plexustyping.png
$top/install.sh
$top/plexustyping
$top/plexustyping-bin
$top/plexustyping.desktop"
expect_eq "archive file list" "$expected_list" \
  "$(tar -tzf "$work/out/$archive_name" | LC_ALL=C sort)"

mkdir -p "$work/x"
tar -xzf "$work/out/$archive_name" -C "$work/x"
pkg="$work/x/$top"
for f in plexustyping plexustyping-bin install.sh; do
  check "$f must be executable" test -x "$pkg/$f"
done
check "plexustyping-bin is the given binary" cmp -s "$fakebin" "$pkg/plexustyping-bin"
for f in LICENSE-MIT LICENSE-APACHE THIRD_PARTY_NOTICES.md; do
  check "$f is the repository file" cmp -s "$repo/$f" "$pkg/$f"
done
check "512x512 icon is src-tauri/icons/icon.png" cmp -s "$repo/src-tauri/icons/icon.png" "$pkg/icons/hicolor/512x512/apps/plexustyping.png"
check "desktop Exec" grep -qx 'Exec=plexustyping' "$pkg/plexustyping.desktop"
check "desktop Icon" grep -qx 'Icon=plexustyping' "$pkg/plexustyping.desktop"
for p in libwebkit2gtk-4.1-0 webkit2gtk4.1 webkit2gtk-4.1 install.sh --uninstall; do
  check "README mentions $p" grep -qF -- "$p" "$pkg/README"
done

# --- launcher: missing WebKitGTK ---------------------------------------------
missing_msg="PlexusTyping necesita WebKitGTK 4.1 (libwebkit2gtk-4.1), que no está instalada.
Instálala con el gestor de paquetes de tu distribución:
  Debian/Ubuntu:  sudo apt install libwebkit2gtk-4.1-0
  Fedora:         sudo dnf install webkit2gtk4.1
  Arch Linux:     sudo pacman -S webkit2gtk-4.1
Después vuelve a abrir PlexusTyping."

set +e
stdout=$(PLEXUSTYPING_LDCONFIG="$work/tools/ldconfig-empty" \
  PLEXUSTYPING_LIB_DIRS="$work/nolibs" "$pkg/plexustyping" a 2>"$work/stderr")
code=$?
set -e
expect_eq "launcher exits 1 when the library is missing" 1 "$code"
expect_eq "launcher missing-library message" "$missing_msg" "$(cat "$work/stderr")"
expect_eq "launcher does not start the binary when the library is missing" "" "$stdout"

# Without a terminal the launcher also shows the message in a window (fake zenity on PATH).
mkdir -p "$work/gui"
printf '#!/bin/sh\nprintf "%%s\\n" "$*" > "%s/gui/zenity.args"\n' "$work" > "$work/gui/zenity"
chmod +x "$work/gui/zenity"
set +e
PLEXUSTYPING_NO_GUI='' PATH="$work/gui:$PATH" PLEXUSTYPING_LDCONFIG="$work/tools/ldconfig-empty" \
  PLEXUSTYPING_LIB_DIRS="$work/nolibs" "$pkg/plexustyping" 2>"$work/stderr-gui" >/dev/null </dev/null
code=$?
set -e
expect_eq "launcher without a terminal still exits 1" 1 "$code"
check "launcher without a terminal shows the message with zenity" grep -q "WebKitGTK 4.1" "$work/gui/zenity.args"

set +e
stdout=$(PLEXUSTYPING_LDCONFIG="$work/tools/no-such-ldconfig" \
  PLEXUSTYPING_LIB_DIRS="$work/nolibs" "$pkg/plexustyping" 2>"$work/stderr")
code=$?
set -e
expect_eq "launcher without ldconfig and no library exits 1" 1 "$code"
expect_eq "launcher without ldconfig: message" "$missing_msg" "$(cat "$work/stderr")"

# --- launcher: library present ------------------------------------------------
stdout=$(PLEXUSTYPING_LDCONFIG="$work/tools/ldconfig-webkit" \
  PLEXUSTYPING_LIB_DIRS="$work/nolibs" "$pkg/plexustyping" "a b" "" "-c" 2>"$work/stderr")
expect_eq "launcher execs the binary with its arguments (ldconfig)" \
  "fake-bin
arg:a b
arg:
arg:-c" "$stdout"
expect_eq "launcher prints nothing on stderr when the library is present" "" "$(cat "$work/stderr")"

stdout=$(PLEXUSTYPING_LDCONFIG="$work/tools/ldconfig-empty" \
  PLEXUSTYPING_LIB_DIRS="$work/nolibs:$work/libs" "$pkg/plexustyping" x)
expect_eq "launcher falls back to the library dirs" "fake-bin
arg:x" "$stdout"

# The launcher works through a symlink and from another directory.
ln -s "$pkg/plexustyping" "$work/link-to-launcher"
stdout=$(cd / && PLEXUSTYPING_LDCONFIG="$work/tools/ldconfig-webkit" "$work/link-to-launcher")
expect_eq "launcher through a symlink" "fake-bin" "$stdout"

# --- install.sh -----------------------------------------------------------------
snapshot() { (cd "$1" && find . -mindepth 1 | LC_ALL=C sort); }

home="$work/home with space"
mkdir -p "$home/.local/bin" "$home/Documents"
echo keep >"$home/.local/bin/other-tool"
echo keep >"$home/Documents/note"
before=$(snapshot "$home")
pkg_before=$(snapshot "$pkg")

set +e
err=$(HOME="$home" "$pkg/install.sh" --bogus 2>&1 >/dev/null); code=$?
set -e
expect_eq "install.sh rejects unknown options with exit 2" 2 "$code"
expect_eq "install.sh usage" "Uso: install.sh [--uninstall]" "$err"
expect_eq "rejected option changes nothing" "$before" "$(snapshot "$home")"

(cd / && HOME="$home" "$pkg/install.sh" >/dev/null)
after=$(snapshot "$home")
new_paths=$(LC_ALL=C comm -13 <(printf '%s\n' "$before") <(printf '%s\n' "$after"))
outside=$(printf '%s\n' "$new_paths" | grep -v '^\./\.local/' || true)
expect_eq "install.sh writes only under \$HOME/.local" "" "$outside"
expect_eq "install.sh removes nothing" "" \
  "$(LC_ALL=C comm -23 <(printf '%s\n' "$before") <(printf '%s\n' "$after"))"
expect_eq "install.sh does not touch the package dir" "$pkg_before" "$(snapshot "$pkg")"

L="$home/.local"
for f in bin/plexustyping bin/plexustyping-bin; do
  check "installed $f executable" test -x "$L/$f"
done
check "installed binary content" cmp -s "$fakebin" "$L/bin/plexustyping-bin"
for s in 32x32 64x64 128x128 256x256 512x512; do
  check "icon $s installed" cmp -s "$pkg/icons/hicolor/$s/apps/plexustyping.png" \
    "$L/share/icons/hicolor/$s/apps/plexustyping.png"
done
desktop="$L/share/applications/plexustyping.desktop"
check "installed desktop Exec points to ~/.local/bin/plexustyping: $(grep '^Exec' "$desktop" || true)" grep -qxF "Exec=\"$L/bin/plexustyping\"" "$desktop"
check "installed desktop Icon" grep -qx 'Icon=plexustyping' "$desktop"

stdout=$(PLEXUSTYPING_LDCONFIG="$work/tools/ldconfig-webkit" "$L/bin/plexustyping" y)
expect_eq "installed launcher runs the installed binary" "fake-bin
arg:y" "$stdout"

# Installing twice, then uninstalling, still leaves nothing behind.
(HOME="$home" "$pkg/install.sh" >/dev/null)
expect_eq "reinstall writes the same files" "$after" "$(snapshot "$home")"

(cd / && HOME="$home" "$pkg/install.sh" --uninstall >/dev/null)
expect_eq "--uninstall restores HOME exactly" "$before" "$(snapshot "$home")"
expect_eq "--uninstall keeps unrelated files" keep "$(cat "$home/.local/bin/other-tool")"

check "--uninstall twice succeeds" env HOME="$home" "$pkg/install.sh" --uninstall >/dev/null
expect_eq "second --uninstall changes nothing" "$before" "$(snapshot "$home")"

# Empty HOME: install creates ~/.local, uninstall leaves the HOME empty again.
home2="$work/home2"
mkdir -p "$home2"
(HOME="$home2" "$pkg/install.sh" >/dev/null)
check "install into an empty HOME" test -x "$home2/.local/bin/plexustyping"
(HOME="$home2" "$pkg/install.sh" --uninstall >/dev/null)
expect_eq "--uninstall leaves an empty HOME empty" "" "$(snapshot "$home2")"

# --- shellcheck -----------------------------------------------------------------
if command -v shellcheck >/dev/null 2>&1; then
  if shellcheck "$packager" "$repo/scripts/linux/plexustyping" \
    "$repo/scripts/linux/install.sh" "${BASH_SOURCE[0]}"; then pass; else fail "shellcheck"; fi
fi

if ((failures > 0)); then
  printf '%d of %d checks failed\n' "$failures" "$checks" >&2
  exit 1
fi
printf 'ok: %d checks passed\n' "$checks"
