# VocaWin implementation plan

This checklist tracks the path from the current local-recognition foundation to a production Windows release. Items are only marked complete when code and a test or CI check exist.

## Model experience

- [x] Model catalog and persistent selected model
- [x] Whisper-family local inference
- [x] ONNX adapters: Parakeet TDT, Moonshine, SenseVoice, GigaAM, Canary
- [~] In-app Whisper download, progress, basic completion check, and deletion (ONNX archives, cancellation, checksums remain)
- [ ] Model disk-use display and hardware recommendations
- [ ] Complete Parakeet CTC and Vosk adapters
- [ ] Model cache migration and resumable downloads

## Dictation experience

- [x] Microphone capture, mono conversion, 16 kHz resampling
- [x] Push-to-talk global shortcut and Windows text injection
- [ ] Toggle activation mode and hotkey re-registration after settings changes
- [ ] Silence/VAD auto-stop, maximum duration, microphone device picker
- [ ] Recording/transcribing tray states, audio level meter, start/stop sound
- [x] Local transcription history with clear-history control
- [ ] Clipboard-preserving paste fallback

## Windows product quality

- [x] Settings persistence and NSIS/MSI Tauri configuration
- [ ] Windows CI compilation job (blocked on AppState/cpal::Stream Send until fixed)
- [ ] System tray, launch at login, single-instance behavior
- [ ] Onboarding, microphone diagnostics, accessible error states
- [ ] Signed release workflow, updater, crash-free upgrade migration

## Verification

- [x] Frontend compilation and Rust unit tests
- [ ] Windows 10/11 manual test matrix: WASAPI devices, NVIDIA/AMD/Intel, UAC targets
- [ ] Automated model fixture tests for each adapter
