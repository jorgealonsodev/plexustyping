#!/usr/bin/env bash
# Installs PlexusTyping from this extracted .tar.gz into ~/.local, for the
# current user only (no root needed):
#   ~/.local/bin/plexustyping            launcher (checks WebKitGTK 4.1)
#   ~/.local/bin/plexustyping-bin        the application binary
#   ~/.local/share/applications/plexustyping.desktop
#   ~/.local/share/icons/hicolor/<size>/apps/plexustyping.png
# It writes nothing outside $HOME/.local. The installed files and the
# directories it had to create are recorded in
# ~/.local/share/plexustyping/install-manifest, so that
#   ./install.sh --uninstall
# removes exactly what was installed.
set -euo pipefail

usage() { echo "Uso: install.sh [--uninstall]" >&2; exit 2; }

mode=install
case $# in
  0) ;;
  1) [[ "$1" == --uninstall ]] || usage; mode=uninstall ;;
  *) usage ;;
esac

if [[ -z "${HOME:-}" || "$HOME" != /* ]]; then
  echo "Error: HOME no está definido o no es una ruta absoluta." >&2
  exit 1
fi

src=$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")
dest="$HOME/.local"
manifest="$dest/share/plexustyping/install-manifest"
icon_sizes=(16x16 24x24 32x32 48x48 64x64 128x128 256x256 512x512)

created_dirs=()
installed_files=()

# Creates $1 (under $dest) and any missing parent, recording each new one.
ensure_dir() {
  local target=$1 path=$dest rest part
  if [[ ! -d "$path" ]]; then mkdir "$path"; created_dirs+=("$path"); fi
  rest=${target#"$dest"/}
  local IFS=/
  for part in $rest; do
    path="$path/$part"
    if [[ ! -d "$path" ]]; then mkdir "$path"; created_dirs+=("$path"); fi
  done
}

# Escapes $1 as a quoted argument of a .desktop Exec key.
desktop_quote() {
  local s=$1
  s=${s//\\/\\\\}
  s=${s//\"/\\\"}
  s=${s//\`/\\\`}
  s=${s//\$/\\\$}
  s=${s//\\/\\\\}
  printf '"%s"' "$s"
}

do_install() {
  local d f size
  # Keep the directories a previous install created, so they are removed too.
  if [[ -f "$manifest" ]]; then
    while IFS=' ' read -r kind path; do
      [[ "$kind" == d ]] && created_dirs+=("$path")
    done <"$manifest"
  fi

  ensure_dir "$dest/bin"
  install -m 755 "$src/plexustyping" "$dest/bin/plexustyping"
  install -m 755 "$src/plexustyping-bin" "$dest/bin/plexustyping-bin"
  installed_files+=("$dest/bin/plexustyping" "$dest/bin/plexustyping-bin")

  ensure_dir "$dest/share/applications"
  f="$dest/share/applications/plexustyping.desktop"
  local exec_line
  exec_line="Exec=$(desktop_quote "$dest/bin/plexustyping")"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" == Exec=* ]]; then
      printf '%s\n' "$exec_line"
    else
      printf '%s\n' "$line"
    fi
  done <"$src/plexustyping.desktop" >"$f"
  chmod 644 "$f"
  installed_files+=("$f")

  for d in "$src"/icons/hicolor/*/apps; do
    size=$(basename "$(dirname "$d")")
    ensure_dir "$dest/share/icons/hicolor/$size/apps"
    f="$dest/share/icons/hicolor/$size/apps/plexustyping.png"
    install -m 644 "$d/plexustyping.png" "$f"
    installed_files+=("$f")
  done

  ensure_dir "$(dirname "$manifest")"
  {
    printf 'd %s\n' "${created_dirs[@]+"${created_dirs[@]}"}" | LC_ALL=C sort -u
    printf 'f %s\n' "${installed_files[@]}"
  } | grep -v '^d $' >"$manifest" || true

  echo "PlexusTyping instalado en $dest/bin/plexustyping"
  case ":${PATH:-}:" in
    *":$dest/bin:"*) ;;
    *) echo "Aviso: $dest/bin no está en el PATH; añádelo para lanzar «plexustyping» desde la terminal." ;;
  esac
  echo "Para desinstalarlo: ./install.sh --uninstall"
}

# Removes $1 only when it is a file under $dest/.
remove_file() {
  [[ "$1" == "$dest"/* ]] || return 0
  if [[ -f "$1" || -L "$1" ]]; then rm -f -- "$1"; fi
}

do_uninstall() {
  local kind path size
  local dirs=()
  if [[ -f "$manifest" ]]; then
    while IFS=' ' read -r kind path; do
      case $kind in
        f) remove_file "$path" ;;
        d) dirs+=("$path") ;;
      esac
    done <"$manifest"
    rm -f -- "$manifest"
    # Deepest first; only directories this script created, and only if empty.
    if ((${#dirs[@]} > 0)); then
      while IFS= read -r path; do
        [[ "$path" == "$dest" || "$path" == "$dest"/* ]] || continue
        rmdir -- "$path" 2>/dev/null || true
      done < <(printf '%s\n' "${dirs[@]}" | LC_ALL=C sort -ru)
    fi
  else
    # No manifest: remove only the files an install would have written.
    remove_file "$dest/bin/plexustyping"
    remove_file "$dest/bin/plexustyping-bin"
    remove_file "$dest/share/applications/plexustyping.desktop"
    for size in "${icon_sizes[@]}"; do
      remove_file "$dest/share/icons/hicolor/$size/apps/plexustyping.png"
    done
  fi
  echo "PlexusTyping desinstalado de $dest."
  echo "Tus datos (sesiones y ajustes) siguen en $dest/share/com.plexustyping.app/."
}

if [[ "$mode" == install ]]; then do_install; else do_uninstall; fi
