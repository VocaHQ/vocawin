# Threat model

VocaWin is an offline voice dictation app for Windows 10/11, built with Tauri 2 (Rust backend in
`src-tauri/src/`, TypeScript frontend in `src/` and `web/`). Hold a hotkey, speak, and the transcript is typed at
the caret. Speech-to-text runs on the user's PC: whisper.cpp (via `transcribe-cpp`) and ONNX Runtime (via
`transcribe-rs` / `ort`). There is no account, no hosted speech API, no telemetry, and no server component.

## Build environment (read this first)
- The product targets Windows only. The scanner image is Linux, so it builds the same crate with
  `cfg(not(windows))`: CPU-only whisper.cpp, no Vulkan, no DirectML, no `windows` crate.
- What you can build and run here: `cd /src/src-tauri && cargo test --offline` (unit tests for the
  command layer, model catalog, download/extract helpers, text post-processing, history, settings, VAD, hotkey
  parsing). `cargo build --offline` builds a Linux binary, but it has no display or audio device here, so it is
  not a useful way to exercise the app.
- What you can only read: everything behind `#[cfg(windows)]`, mainly `src-tauri/src/output.rs` (text injection
  via SendInput / clipboard), `hook.rs` (low-level keyboard hook), `ducking.rs`, `autostart.rs` (registry Run
  key), `power.rs`, `gpu.rs`, the NSIS hooks in `src-tauri/windows/`. These are in scope. Please reason about them
  from source and say clearly in the report that a finding was not reproduced on Windows.

## What this project does and where untrusted input enters
1. **Model downloads (highest priority).** `download_model` / `download_url_to_file` / the tar.gz extractor in
   `src-tauri/src/lib.rs` fetch models over HTTPS from Hugging Face and `blob.handy.computer`, then unpack
   archives into the user's models directory. Treat the remote server, the HTTP response, and the archive
   contents as attacker-controlled (compromised mirror or CDN). Path traversal, symlink/hardlink entries,
   writes outside the models directory, and unbounded resource use are all in scope.
2. **Model files on disk.** Whisper `.bin`/GGUF, `.onnx`, tokenizer/vocab files, and the Silero VAD model are
   parsed by whisper.cpp and ONNX Runtime. Bugs in VocaWin's own handling of these files are in scope; bugs
   that live purely inside upstream whisper.cpp / ONNX Runtime are lower priority (report them, flagged as
   upstream).
3. **The webview → Rust boundary.** The frontend calls about 45 `#[tauri::command]` handlers. Assume webview
   content could be attacker-influenced (e.g. via a transcript or model name rendered without escaping), so
   command arguments are untrusted: file paths, model ids, URLs (`open_external` has an allow list), settings.
4. **Transcribed text.** The speech recogniser's output is effectively attacker-controlled (anyone who can
   play audio near the mic). It flows through post-processing (`cleanup.rs`, `spoken_numbers.rs`,
   `spoken_emoji.rs`, `dictionary.rs`, `vocabulary.rs`, `hinglish.rs`), into history storage, into the UI,
   and is then typed into whatever window has focus. Injection into the UI (XSS in the webview), and anything
   that makes typed text trigger unintended key combinations or commands, is in scope.
5. **Local files the app reads back:** settings JSON, history, user dictionary/vocabulary. Lower priority:
   other local processes running as the same user are out of scope as attackers.

## Components that matter most / least
- Most: model download/extract, Tauri command handlers and the CSP, text injection (`output.rs`), the
  webview's rendering of transcripts and history.
- Less: tray, sounds, stats, overlay layout, the static marketing site in `web/`.
- Out of scope: third-party crates and C/C++ libraries themselves, unless VocaWin uses them unsafely;
  `.github/` workflows; `docs/`.

## How to exercise it
- `cd /src/src-tauri && cargo test --offline` runs the unit tests. Add your own `#[test]` cases next to the
  code under test to write a reproducer (for example a crafted tar.gz for the extractor).
- Fixtures live in `src-tauri/tests/fixtures/`.

## How you rate severity
- Critical: code execution or arbitrary file write from a malicious or compromised model server/archive with
  no user action beyond picking a model; any way for webview content to run arbitrary commands.
- High: file write or delete outside the app's data directories; Tauri command misuse that reads arbitrary
  files or opens arbitrary URLs/programs; transcribed speech that can trigger keystrokes beyond literal text.
- Medium: XSS in the webview with no path to a Tauri command; integrity gaps (e.g. a model not verified by
  checksum) without a demonstrated exploit; crashes/panics from malformed model files or downloads.
- Low: denial of service needing local access, log noise, hardening suggestions.
- Memory-safety bugs in VocaWin's own `unsafe` blocks are at least High if reachable from untrusted input.

## Reports and patches
- Short root cause, the input that triggers it, a `cargo test` reproducer if possible, and a minimal patch in
  the existing code style. Say whether the issue was reproduced on Linux or reasoned from Windows-only source.

## Anything to leave alone
- Unsigned installers and SmartScreen warnings are known and documented.
- No CUDA build exists by design; GPU paths fall back to CPU.
- RUSTSEC-2024-0429 (GTK glib) only affects non-Windows targets and is ignored on purpose.
