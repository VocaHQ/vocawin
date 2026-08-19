# VocaWin implementation plan

This checklist tracks the path from the current local-recognition foundation to a production Windows release. Items are only marked complete when code and a test or CI check exist.

## Model experience

- [x] Model catalog and persistent selected model
- [x] Whisper-family local inference
- [x] ONNX adapters: Parakeet TDT, Moonshine, SenseVoice, GigaAM, Canary
- [x] In-app Download for every catalog model (Whisper GGML + ONNX archives/files), progress, install check, and Remove
- [x] Hardware recommendation for a starting model size
- [x] Failed downloads stay Failed (never Complete/Installed)
- [x] Installed models show on-disk size
- [ ] Parakeet CTC and Vosk adapters (kept out of the catalog until they work)
- [ ] Model cache migration, resumable downloads, and checksums

## Dictation experience

- [x] Microphone capture, mono conversion, 16 kHz resampling
- [x] Microphone device picker (WASAPI / cpal list)
- [x] Push-to-talk and toggle activation, with hotkey re-registration after save
- [x] Hotkey presets + Record capture (Escape cancels); WH_KEYBOARD_LL for lone Right Ctrl/Alt/Shift; listener pauses while Record
- [x] New-install default hotkey Right Alt (VocaLinux hold-default; AltGr left alone). Existing TROOPER Ctrl+Alt+Space settings kept until changed
- [x] Silence energy auto-stop in toggle mode only (PTT stops on key-up); `silence_seconds` wired
- [x] Trailing space and auto-capitalize output polish
- [x] Clipboard + paste inject with restore (SendInput fallback)
- [x] Start/stop/error sound cues when enabled (PlaySound WAV; not silent Beep)
- [x] Local transcription history with clear-history control
- [x] Tray idle / listening / processing icons and full tray menu
- [x] Settings search + Mic Test (level) + Test Dictation (recognize, no inject)
- [x] Searchable language list + Auto-detect
- [x] No-model honesty (never “no speech was recognized” when none installed)

## Windows product quality

- [x] Settings persistence and NSIS/MSI Tauri configuration
- [x] Windows CI compilation job (Vulkan SDK + Ninja generator for whisper-rs)
- [x] NSIS/MSI installer artifact job (alpha/dev; main/workflow_dispatch only, PRs cargo-test)
- [x] Tag-triggered GitHub Release for testers (`v*` tags, always prerelease)
- [x] Official Voca mic app/tray icons (brand book §9 / §10)
- [x] System tray menu: Start/Stop Voice Typing, Start on Login, Settings, View Logs, About, Quit
- [x] Launch at login and single-instance focus (`--start-minimized` for login / CLI)
- [x] Whisper Vulkan on Windows builds only; catalog/system_summary match compile-time reality; prefer discrete, skip WARP
- [x] Auto-pause while listed apps run (opt-in)
- [x] Idle Whisper unload keep-alive (opt-in, default 300s)
- [x] Sleep/wake hotkey recovery (WM_POWERBROADCAST)
- [x] Short first-run welcome (tray + hold hotkey + optional login)
- [x] Reusable Logs window (single instance)
- [ ] Signed release workflow, updater, crash-free upgrade migration

## Verification

- [x] Frontend compilation and Rust unit tests
- [ ] Windows 10/11 manual test matrix: WASAPI devices, NVIDIA/AMD/Intel, UAC targets
- [ ] Automated model fixture tests for each adapter
