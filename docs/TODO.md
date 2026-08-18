# VocaWin implementation plan

This checklist tracks the path from the current local-recognition foundation to a production Windows release. Items are only marked complete when code and a test or CI check exist.

## Model experience

- [x] Model catalog and persistent selected model
- [x] Whisper-family local inference
- [x] ONNX adapters: Parakeet TDT, Moonshine, SenseVoice, GigaAM, Canary
- [x] In-app Download for every catalog model (Whisper GGML + ONNX archives/files), progress, install check, and Remove
- [ ] Model disk-use display and hardware recommendations
- [ ] Parakeet CTC and Vosk adapters (kept out of the catalog until they work)
- [ ] Model cache migration, resumable downloads, and checksums

## Dictation experience

- [x] Microphone capture, mono conversion, 16 kHz resampling
- [x] Push-to-talk and toggle activation, with hotkey re-registration after save
- [x] Hotkey presets + Record capture (Escape cancels)
- [x] Silence energy auto-stop and max recording duration
- [x] Trailing space and auto-capitalize output polish
- [x] Clipboard-preserving paste fallback when SendInput fails
- [ ] Microphone device picker and start/stop sound cues
- [x] Local transcription history with clear-history control
- [x] Recording tray tooltip state

## Windows product quality

- [x] Settings persistence and NSIS/MSI Tauri configuration
- [x] Windows CI compilation job (Vulkan SDK for whisper-rs)
- [x] Unsigned NSIS/MSI installer artifact job (alpha/dev, no signing or Release)
- [x] Official Voca mic app/tray icons (brand book §9 / §10)
- [x] System tray icon with Show/Quit and close-to-tray
- [x] Launch at login and single-instance focus
- [x] Whisper Vulkan GPU path + Settings GPU readout (DirectML kept for ONNX)
- [ ] Onboarding, microphone diagnostics, accessible error states
- [ ] Signed release workflow, updater, crash-free upgrade migration

## Verification

- [x] Frontend compilation and Rust unit tests
- [ ] Windows 10/11 manual test matrix: WASAPI devices, NVIDIA/AMD/Intel, UAC targets
- [ ] Automated model fixture tests for each adapter
