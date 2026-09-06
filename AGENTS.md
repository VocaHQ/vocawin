# AGENTS.md

Instructions for coding agents on **VocaWin** (`VocaHQ/vocawin`). Default branch is `main`. License: AGPL-3.0-or-later.

Native Windows voice typing: hold a hotkey, speak, text at the caret. After a Whisper or ONNX model is on disk, capture and transcription stay on this PC. No Voca account, no hosted speech API, no product telemetry, no gateway mode.

This is a **beta**. Installers are **unsigned**. Do not call it signed, Store-ready, stable, or a public ship.

## Critical: git worktrees for every branch and PR

Never create a branch, commit, or open a pull request in the primary checkout. Always use a linked git worktree so the main working tree stays on `main` and stays clean. Do not `git switch` / `git checkout` a feature branch in the primary directory, and do not leave it dirty.

```bash
git fetch origin
git worktree add /tmp/vocawin-<task> -b <type>/<short-name> origin/main

# All edits, commits, and `gh pr create` happen inside that worktree.

git worktree remove /tmp/vocawin-<task>
git worktree prune
```

Rules:

- One worktree per branch, one branch per PR
- Place worktrees **outside** the primary working tree (`/tmp/vocawin-<task>` or a sibling directory such as `../.worktrees/vocawin-<task>`)
- Never run two tasks in the same worktree
- Never commit directly to `main`
- Clean up the worktree after the PR is pushed

## Stack

| Layer | Location | Notes |
| --- | --- | --- |
| Tauri 2 shell | `src-tauri/` | Identifier `com.vocahq.vocawin`. Bundles NSIS + MSI. |
| TypeScript UI | `src/` | Vanilla TS. `src/main.ts` + `src/style.css`. No React. Vite on port **1420**. |
| Rust | `src-tauri/src/` | Crate `vocawin_lib`. Commands, tray, capture, STT, inject. |
| Landing page | `web/` | Static vocawin.com. No build step. GitHub Pages. |

Rust modules (keep work in the matching file):

| File | Role |
| --- | --- |
| `lib.rs` | Catalog, settings, recording, ONNX adapters, Tauri commands, tray |
| `hotkey.rs` | Presets and parse (`AltRight` default). Rejects Win/Super. |
| `hook.rs` | `WH_KEYBOARD_LL`. Lone Right Alt/Ctrl/Shift. Leaves AltGr alone. |
| `devices.rs` | WASAPI mic list via cpal |
| `whisper_cache.rs` | whisper.cpp keep-alive + optional idle unload |
| `gpu.rs` / `hardware.rs` | DXGI GPU pick (skip WARP) and starting-model hint |
| `output.rs` | `SendInput` first (clipboard left alone); clipboard paste + restore fallback; optional copy-to-clipboard |
| `autopause.rs` / `power.rs` | Opt-in app pause; sleep/wake hotkey rebind |
| `sounds.rs` / `logbuf.rs` | PlaySound themes; in-memory logs (not a file) |

Windows builds enable `whisper-rs` **Vulkan** and `transcribe-rs` **DirectML**. Non-Windows keeps CPU-only whisper so `cargo test` still runs. `build.rs` sets `cfg(vocawin_whisper_vulkan)` only for Windows targets — catalog strings must match that cfg.

App data: `%APPDATA%\com.vocahq.vocawin\` (`settings.json`, `history.json`, `models/`). Window title says Beta. Close hides to tray; Quit is tray-only. Autostart uses `--start-minimized`.

## Commands

Prereqs: Node 20+ (CI uses 22), Rust stable, and [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) (MSVC + WebView2) for a real desktop build.

| Command | What |
| --- | --- |
| `npm install` | Frontend deps |
| `npm run dev` | Vite only (`tauri.conf.json` `beforeDevCommand`) |
| `npm run build` | `tsc --noEmit && vite build` |
| `npm run tauri` | Tauri CLI passthrough |
| `npm run tauri dev` | Desktop development |
| `npm run tauri build` | NSIS `.exe` + MSI on Windows |
| `npm run check` | Frontend build + `cargo test --manifest-path src-tauri/Cargo.toml` |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust unit tests (Linux/macOS OK) |

macOS/Linux can validate UI compile and the Rust command layer. WASAPI, Vulkan, DirectML, injection, and installers need Windows 10/11.

Website:

```bash
cd web
python3 -m http.server 4173 --directory .
node --test tests/site.test.mjs
```

## Architecture

```text
src/main.ts (dictation / models / history / settings; #logs window)
  └─ Tauri commands in src-tauri/src/lib.rs
      ├─ Settings + model catalog + download/unpack
      ├─ WH_KEYBOARD_LL hotkey + recording coordinator
      ├─ cpal WASAPI capture, mono, resample to 16 kHz
      ├─ whisper.cpp (GGML .bin) via whisper_cache
      ├─ ONNX (Parakeet TDT, Moonshine, SenseVoice, GigaAM, Canary)
      ├─ Tray (idle / listening / processing)
      └─ inject: SendInput (no clipboard); clipboard Ctrl+V + restore fallback
```

Do not add a frontend framework. Add Tauri commands next to the existing `invoke_handler` list and call them with `invoke()` from `src/main.ts`.

## Privacy and models

- After the first model download, audio and transcription do not leave the PC.
- No Voca cloud. No telemetry. Do not add analytics, crash reporters, or updater JSON.
- Installer does **not** bundle a model. Prefer in-app Download on the Models page.
- Layout and adapter IDs: `docs/MODELS.md`. Keep `parakeet-ctc-*` and `vosk-*` out of the catalog until they transcribe.
- Whisper: `%APPDATA%\com.vocahq.vocawin\models\<id>.bin`. ONNX: a directory named by catalog id.
- GPU: whisper.cpp Vulkan with CPU fallback; ONNX DirectML with CPU fallback (`ort-directml` on Windows). DXGI skips software/WARP adapters.

## Windows pitfalls (verified)

- **Unsigned / SmartScreen** — NSIS and MSI have no purchased CA. Windows will say the publisher is unknown. That is expected. Do not claim a Store listing or a signed stable.
- **Elevated windows** — UIPI can block clipboard/`SendInput` injection into admin targets. Documented in README and `web/`; do not promise it works there.
- **GPU** — Vulkan (whisper.cpp) and DirectML (ONNX) with CPU fallback. Catalog labels must follow `cfg(vocawin_whisper_vulkan)`.
- **Hotkeys** — `RegisterHotKey` cannot bind a lone modifier; the LL hook does. AltGr (Ctrl+Right Alt) must not be consumed. Default is Right Alt (`AltRight`).
- **CI path length** — Windows jobs set `CARGO_TARGET_DIR=C:\t` and `CMAKE_GENERATOR=Ninja` because whisper.cpp Vulkan shader nests hit `MAX_PATH`. Do not switch those jobs back to the default `target/` + MSBuild without a proven fix.
- **Self-sign** — Authenticode timestamping was dropped after hangs. Do not reintroduce a signing step in `windows-ci.yml`.

## Releases and CI

Keep the app version and the public Git tag aligned (`0.1.1` / `v0.1.1` for this cut) in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Bump both together on a named cut. Do not bump the app version for a nightly alone. Do not pin a `v*` tag in README; vocawin.com names the current tagged cut and updates when that cut changes.

| Workflow | Trigger | Effect |
| --- | --- | --- |
| `windows-ci.yml` | `main` push, PRs, dispatch | Source-change filter. PRs: `cargo test` only. `main`/dispatch: unsigned NSIS+MSI **artifact**. Docs-only PRs skip the Windows VMs so the required check is not left pending. |
| `windows-alpha-release.yml` | `v*` tag only | Unsigned NSIS+MSI on a **prerelease**. No `workflow_dispatch`. `includeUpdaterJson: false`. |
| `nightly.yml` | cron + dispatch | Moving `nightly` prerelease from `main` when app source changed. Not a `v*` tag. |
| `deploy-pages.yml` | `web/**` on `main` | Publishes vocawin.com. |

Named tester cuts: `v*` via `RELEASE.md` (latest prep is `v0.1.1`). Testers use GitHub Releases, not the CI artifact. NSIS is current-user; MSI is the WiX wizard. Use one installer per PC, not both.

Do not set `tagName` in `windows-ci.yml`. Do not enable an updater. Do not retag. Nightly may be dispatched; a named beta must not be cut from a branch click.

## Website (`web/`)

Static HTML/CSS/JS. Hero download goes to `/releases`, with a quieter nightly link. Site pins name the current tagged cut. `web/tests/site.test.mjs` enforces honesty: beta, unsigned, SmartScreen, current tag pin, no "coming soon", no telemetry snippets, AGPL-3.0-or-later.

If you change landing copy, run `node --test tests/site.test.mjs` from `web/`. Do not edit the VocaHQ directory repo from here.

## Git and PRs

- Conventional commits: `docs:`, `feat:`, `fix:`, `chore:`, `ci:`, `refactor:`. Imperative subject. Example: `docs: add AGENTS.md with mandatory git worktrees`.
- Never push to `main`. Never commit on `main`. Never merge the PR yourself.
- Open against `main` with `gh pr create` from the worktree.
- Docs-only PRs should not need a Windows package job; keep app-source path filters intact.
- Do not duplicate README marketing. Product facts live in README, `docs/setup.md`, `docs/MODELS.md`, and `RELEASE.md`.

## Do not

- Claim Store, signed, stable, auto-update, or “100% offline” (the first model download uses the network).
- Add Voca cloud, gateway mode, telemetry, or `latest.json`.
- Offer a catalog Download that cannot transcribe.
- Change installer copy in `src-tauri/windows/nsis-hooks.nsh` without matching README honesty.
- Leave the primary checkout on a feature branch or dirty.
