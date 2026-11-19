# Phase 4 · Linux packages (.deb and .tar.gz)

Ship the two Linux artefacts of PRD §7 «Distribución y actualizaciones» from CI, with checksums. Windows signing and the updater are out of scope: they need a certificate budget (PRD §9), a human decision. Branch `feat/linux-packages`.

## Specs

- **S1 — .deb.** PRD:

  > `plexustyping_<versión>_amd64.deb` · Lo genera el bundler de Tauri (`bundle.targets: ["deb"]`). Declara las dependencias `libwebkit2gtk-4.1-0` y `libgtk-3-0`, e instala el lanzador `.desktop` y los iconos.

  Bundle targets are limited to the v1 platforms: `deb` on Linux and `nsis` on Windows. AppImage, msi, dmg and app are no longer built. The `.deb` control file declares those two dependencies.
- **S2 — .tar.gz.** PRD:

  > `plexustyping-<versión>-linux-x86_64.tar.gz` · No lo genera el bundler de Tauri: lo empaqueta un script de CI. Lleva el binario, los recursos (corpus, layouts, temas), `plexustyping.desktop`, los iconos, `LICENSE`, `CORPUS-LICENSES.md` y un `README` con las dependencias del sistema y un `install.sh` opcional para instalar en `~/.local`.

  The resources are already compiled into the binary (`include_str!`), so the archive carries no separate resource files.

  The repository has `LICENSE-MIT`, `LICENSE-APACHE` and `THIRD_PARTY_NOTICES.md` instead of `LICENSE` and `CORPUS-LICENSES.md`. Those existing files are shipped.

  The script is a repository script (`scripts/package-linux-tarball.sh <binary> <version> <outdir>`). It is run by CI and is testable locally.

  `install.sh` installs into `~/.local`: the binary under `~/.local/bin`, the `.desktop` file under `~/.local/share/applications` and the icons under `~/.local/share/icons/hicolor`. It writes nothing outside `$HOME/.local`. A matching uninstall option (`install.sh --uninstall`) removes exactly what it installed.
- **S3 — Missing WebKitGTK.** PRD:

  > el binario comprueba al arrancar que existan y muestra un mensaje claro si faltan

  A dynamically linked binary cannot run its own check when `libwebkit2gtk-4.1` is missing, because the loader fails first. So the `.tar.gz` starts the app through a launcher script, `plexustyping`, which does the check. The real binary sits next to it.

  When the library is missing, the launcher prints a clear Spanish message naming the package for Debian/Ubuntu, Fedora and Arch and exits with status 1. The tarball README documents the same packages.
- **S4 — Checksums.** PRD: "Cada release publica sumas `SHA256SUMS` de todos los artefactos." CI writes `SHA256SUMS` covering every published Linux artefact.
- **S5 — CI.** PRD: "generadas en CI con runners propios de cada sistema (`ubuntu-22.04` …)."
  - A GitHub Actions workflow builds on `ubuntu-22.04` when a `v*` tag is pushed, or when run by hand. It does not run on ordinary pushes.
  - It runs the frontend and Rust tests, builds the `.deb`, packages the `.tar.gz`, writes `SHA256SUMS` and uploads all three as workflow artefacts.
  - On a tag it attaches them to a draft GitHub release. Publishing the release stays a human action.
  - Every third-party action is pinned to a full commit SHA.
- **S7 — Windows installer and release (L4).**
  - The workflow also builds `PlexusTyping_<versión>_x64-setup.exe` (NSIS, per-user, PRD §7) on `windows-latest`. It is unsigned, so SmartScreen warns: PRD: "sin firma, aparece el aviso de SmartScreen".
  - The release attaches the `.deb`, the `.tar.gz`, the `.exe` and one `SHA256SUMS` covering all of them.
  - After the workflow is merged, the parent pushes tag `v0.1.0`. Once the jobs pass, the parent publishes the release.
  - The updater and signing stay out of scope.
- **S6 — Docs.** README explains how to install each artefact and the system dependencies.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1–S6 | Bundle targets, tarball script with launcher, install.sh (+ --uninstall), tarball README, a shell test of the script, release workflow, README | worker | done | see L5 |
| T3 | S5, S7 | Merge, tag v0.1.0, watch the workflow, publish the release | parent | pending | |
| T2 | S1–S4 | Local proof: real `tauri build --bundles deb`, `dpkg-deb -I` dependencies, the script on the real binary, launcher message with the library hidden, install/uninstall into a temporary HOME | parent | pending | |

## Log

- **L1 (user, verbatim, 2026-11-19):** "haz lo que consideres mas optimo" (after the agent proposed the Linux installers as the next phase 4 block).
- **L2 (evidence, 2026-11-19):** the repository state before this work.
  - `tauri.conf.json` builds deb, appimage, msi, nsis, dmg and app.
  - There is no `.github/workflows`, no tarball script and no checksums.
  - Layouts and corpora are embedded with `include_str!`.
  - The licences are LICENSE-MIT, LICENSE-APACHE and THIRD_PARTY_NOTICES.md.
  - The bin is named `plexustyping`, the version is 0.1.0 and the bundle id is `com.plexustyping.app`.
- **L3 (agent decisions, 2026-11-19):**
  - The S3 check lives in a launcher script, because a binary cannot report a missing library it is linked against.
  - The existing licence files are shipped under their own names.
  - The workflow only drafts releases; publishing is the human's call.
  - Windows signing and the updater wait for the certificate decision.
- **L4 (user, verbatim, 2026-11-19):** "habra que hacer release, crea los instaladores en github" → S7 added. The unsigned Windows NSIS installer is included, and the release v0.1.0 is created and published by the parent once CI passes.
- **L5 (evidence, 2026-11-19):** T1 done by the worker plus parent fixes.
  - Scripts: `scripts/package-linux-tarball.sh`, `scripts/linux/{plexustyping,install.sh,plexustyping.desktop,README}` and `scripts/test-package-linux-tarball.sh`. RED: exit 127. GREEN: 57 checks, shellcheck clean.
  - Workflow: `.github/workflows/release.yml` (linux, windows and release jobs; draft release; actions pinned to commit SHAs). README has an install section.
  - Parent fixes:
    - Tauri names the .deb `PlexusTyping_<v>_amd64.deb` with Debian package `plexus-typing` (there is no packageName option). The workflow reads that file and publishes it as `plexustyping_<v>_amd64.deb`. README says `apt remove plexus-typing`.
    - Bundle metadata added (publisher, homepage, license, category, short and long description).
    - WebView2 set to `embedBootstrapper` (PRD: "Incluye el bootstrapper de WebView2").
    - The explicit deb depends were removed: Tauri already adds `libwebkit2gtk-4.1-0, libgtk-3-0`, and listing them again duplicated them. The workflow still checks both with `dpkg-deb -f`.
  - T2 local proof:
    - A real `tauri build --bundles deb` gives `Depends: libwebkit2gtk-4.1-0, libgtk-3-0`, with the .desktop file and the icons.
    - The tarball built from the real binary has the expected file list.
    - With the library hidden, the launcher prints the Spanish message and exits 1.
  - Checks: cargo 586, vitest 412, check 0.
  - Not yet run: the workflow itself (the Windows build and the ubuntu-22.04 package list).
- **L6 (review, 2026-11-19):** review-de5cd96b7a9b4814 (risk, resilience and reliability) approved 3222e3e with advisories. Two were applied:
  1. R3-cargo-test-before-frontend-build: the linux job now runs `pnpm build` before `cargo test`, because `generate_context!` needs `dist/`. Locally it passed only because `dist/` already existed.
  2. R4-launcher-silent-menu-failure: without a terminal, the launcher also shows the message with zenity, kdialog or notify-send. Test with a fake zenity; `PLEXUSTYPING_NO_GUI` disables it. Tarball test 59/59, shellcheck clean.

  Still open: `%` in a HOME path is not escaped in `Exec`; a failed partial install leaves its directories; there is no relative-path test of the packager.
