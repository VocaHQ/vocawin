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
- [x] Push-to-talk global shortcut and Windows text injection
- [ ] Toggle activation mode and hotkey re-registration after settings changes
- [ ] Silence/VAD auto-stop, maximum duration, microphone device picker
- [ ] Recording/transcribing tray icon states, audio level meter, start/stop sound
- [x] Local transcription history with clear-history control
- [ ] Clipboard-preserving paste fallback

## Windows product quality

- [x] Settings persistence and NSIS/MSI Tauri configuration
- [x] Windows CI compilation job
- [x] Unsigned NSIS/MSI installer artifact job (alpha/dev, no signing or Release)
- [x] System tray icon with Show/Quit and close-to-tray
- [ ] Launch at login and single-instance behavior
- [ ] Onboarding, microphone diagnostics, accessible error states
- [ ] Signed release workflow, updater, crash-free upgrade migration

## Verification

- [x] Frontend compilation and Rust unit tests
- [ ] Windows 10/11 manual test matrix: WASAPI devices, NVIDIA/AMD/Intel, UAC targets
- [ ] Automated model fixture tests for each adapter
