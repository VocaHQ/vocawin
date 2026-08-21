<p align="center">
  <img src="web/assets/brand/voca-logo-512.png" alt="VocaWin" width="128" height="128">
</p>

<h1 align="center">VocaWin</h1>

<p align="center"><strong>Voice-to-text for Windows.</strong></p>

<div align="center">

[![Release](https://img.shields.io/github/v/release/VocaHQ/vocawin?include_prereleases)](https://github.com/VocaHQ/vocawin/releases)
[![Status](https://img.shields.io/badge/status-developer%20alpha-yellow)](#development)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey)](#system-requirements)
[![Privacy](https://img.shields.io/badge/privacy-on%20this%20PC-success)](#key-principles)

[![License](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Website](https://img.shields.io/badge/Website-vocawin.com-informational)](https://vocawin.com)
[![Discord](https://img.shields.io/discord/1538633755877580810?logo=discord&logoColor=white&label=Discord)](https://discord.gg/t6muquAJbm)
[![Follow us](https://img.shields.io/badge/Follow%20us-000000?logo=x&logoColor=white)](https://x.com/vocahq)
[![VocaHQ](https://img.shields.io/badge/VocaHQ-vocahq.com-1a7f4e)](https://vocahq.com)

Developer alpha. Unsigned. Testers can grab a build from [GitHub Releases](https://github.com/VocaHQ/vocawin/releases). This is not a signed store build and not a stable public release.

</div>

---

## What is VocaWin?

VocaWin is native Windows voice typing. After a Whisper or ONNX model is on disk, recording and speech-to-text stay on this PC. No Voca account, no hosted speech API.

It sits next to [VocaLinux](https://vocalinux.com), [VocaMac](https://vocamac.com), and [VocaPhone](https://vocaphone.vocahq.com). The family directory is [vocahq.com](https://vocahq.com).

## Key Principles

- **On this PC** - After the model download, transcription is designed to run locally
- **No Voca cloud** - There is no hosted speech service to sign up for
- **Open source** - The repository is public
- **No telemetry in the product** - The app does not phone home
- **Windows native** - Tray app, hotkey, WASAPI, text at the caret
- **Honest status** - Developer alpha. Unsigned. Windows will likely say the publisher is unknown.

## Try it

Testers can install an unsigned developer alpha today. Download the NSIS `.exe` or the MSI from [GitHub Releases](https://github.com/VocaHQ/vocawin/releases). Windows will likely say the publisher is unknown. That is SmartScreen. More info, then Run anyway if you trust the file. Read [the setup guide](docs/setup.md) first.

This is a tester build you can run today. It is not a store listing and not a stable public release.

### What works

- **Hold a hotkey, speak, text at the caret** - Default is Right Alt, the same hold as VocaLinux. Double-tap toggles. You can change the hotkey in Settings.
- **Tray** - Idle, recording, and processing icon states. Close goes to the tray. Show window / Quit.
- **Settings** - Hotkeys, models, languages, silence detection, sounds, start on login.
- **Local models** - In-app Download for Whisper/whisper.cpp, Distil-Whisper, Parakeet, Moonshine, SenseVoice, GigaAM, and Canary.
- **GPU** - whisper.cpp on Vulkan with CPU fallback. ONNX Runtime on DirectML with CPU fallback.
- **Clipboard restore** after injection.

### Still rough

- Unsigned. SmartScreen is expected. There is no purchased CA signature and no Microsoft Store listing.
- The installer does not bundle a speech model. First run needs a network once to download one.
- Elevated windows can block text injection.
- Parakeet CTC and Vosk stay out of the catalog until they work.
- No auto-update. Expect bugs. [File an issue](https://github.com/VocaHQ/vocawin/issues) if something breaks.

The recognizer is not a cloud API. VocaWin only invokes a locally installed or downloaded engine. Model downloads may use the network once. Audio and transcription do not.

### Architecture

```text
Tauri UI (TypeScript)
  └─ Rust command layer
      ├─ Settings + model catalog
      ├─ Global push-to-talk shortcut + UI recording coordinator
      ├─ CPAL microphone capture + 16 kHz resampling
      ├─ whisper.cpp adapter (Whisper-family models)
      ├─ ONNX adapters (Parakeet, Moonshine, SenseVoice, GigaAM, Canary)
      ├─ System tray (Show / Quit, close-to-tray)
      └─ Windows text injector (SendInput)
```

| Engine | Initial models | Windows acceleration |
| --- | --- | --- |
| whisper.cpp | Tiny through Large v3 Turbo, Distil-Whisper | Vulkan, CPU fallback |
| ONNX Runtime | Parakeet, Moonshine, SenseVoice, GigaAM, Canary | DirectML, CPU fallback |

### Local model setup (developers)

The development build does not bundle a large model. Prefer the in-app Download buttons on the Models page. For Whisper testing from a shell, you can also place a whisper.cpp GGML model into VocaWin's local model folder:

```powershell
$models = Join-Path $env:APPDATA "com.vocahq.vocawin\models"
New-Item -ItemType Directory -Force $models
Invoke-WebRequest "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin" -OutFile (Join-Path $models "whisper-tiny.bin")
```

See [the ONNX model guide](docs/MODELS.md) for Parakeet, Moonshine, SenseVoice, GigaAM, and Canary layouts.

## The Voca ecosystem

Same privacy bar, different machines. Start at [vocahq.com](https://vocahq.com) for the map.

| Platform | Project | Website | GitHub | Status |
|----------|---------|---------|--------|--------|
| Family | **VocaHQ** | [vocahq.com](https://vocahq.com) | [VocaHQ](https://github.com/VocaHQ) | Directory |
| Linux | **VocaLinux** | [vocalinux.com](https://vocalinux.com) | [VocaHQ/vocalinux](https://github.com/VocaHQ/vocalinux) | Available |
| macOS | **VocaMac** | [vocamac.com](https://vocamac.com) | [VocaHQ/vocamac](https://github.com/VocaHQ/vocamac) | Beta |
| iPhone / Android | **VocaPhone** | [vocaphone.vocahq.com](https://vocaphone.vocahq.com) | [VocaHQ/vocaphone](https://github.com/VocaHQ/vocaphone) | Beta / source build |
| Windows | **VocaWin** | [vocawin.com](https://vocawin.com) | [VocaHQ/vocawin](https://github.com/VocaHQ/vocawin) | Developer alpha |
| Infrastructure | **VocaGateway** | | [VocaHQ/vocagateway](https://github.com/VocaHQ/vocagateway) | Early |

VocaGateway is optional self-hosted compute for other Voca clients. VocaWin does not expose a gateway mode today.

## Tech Stack

- **Speech Engine**: [whisper.cpp](https://github.com/ggerganov/whisper.cpp) and ONNX Runtime, both local
- **Platform**: Windows 10/11
- **GPU**: Vulkan for Whisper, DirectML for ONNX, CPU fallback for both
- **Languages**: determined by the downloaded model

## Development

Prerequisites: Node.js 20+, Rust stable, and the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) (Microsoft C++ Build Tools and WebView2) for a Windows build.

```bash
npm install
npm run tauri dev       # desktop development
npm run tauri build     # creates NSIS and MSI artifacts on Windows
npm run check           # TypeScript build + Rust tests
```

A macOS/Linux host can validate the frontend and Rust command layer, but Windows injection and installer artifacts must be exercised on Windows 10/11.

Windows CI builds an NSIS (and MSI) installer on pushes to main and on workflow_dispatch, then uploads it as a GitHub Actions artifact. Pull requests only run cargo test, so a docs change does not package the setup wizards. The installers stay unsigned. SmartScreen can still warn.

Pushing a `v*` tag builds the same NSIS and MSI and attaches them to a GitHub Release marked as a prerelease. Testers should use [Releases](https://github.com/VocaHQ/vocawin/releases), not the workflow artifact. The build is unsigned, not a purchased CA or store signature. Windows will likely still warn. More info, then Run anyway. There is no Microsoft Store listing and no auto-update. Read [the setup guide](docs/setup.md) before you install, and [file an issue](https://github.com/VocaHQ/vocawin/issues) if something breaks. [vocawin.com](https://vocawin.com) points at the same download.

## System Requirements

- Windows 10 version 1809 or later, or Windows 11
- 4 GB RAM (8 GB+ recommended for larger models)
- Microphone
- GPU recommended for faster transcription (NVIDIA, AMD, or Intel)

## Website

The landing page at [vocawin.com](https://vocawin.com) lives in `web/` and deploys through GitHub Pages.

```bash
cd web
python3 -m http.server 4173
node --test tests/site.test.mjs
```

GitHub Actions publishes `web/` on pushes to `main`. The `web/CNAME` file maps `vocawin.com`.

## Contributing

VocaWin is in early development. Download the unsigned alpha from [Releases](https://github.com/VocaHQ/vocawin/releases) if you want to try it. File bugs on [Issues](https://github.com/VocaHQ/vocawin/issues). The family directory is [vocahq.com](https://vocahq.com).

## Project references

The product design and model support are informed by [VocaMac](https://github.com/VocaHQ/vocamac), [VocaLinux](https://github.com/VocaHQ/vocalinux), [VocaPhone](https://github.com/VocaHQ/vocaphone), [VocaGateway](https://github.com/VocaHQ/vocagateway), [Handy](https://github.com/cjpais/Handy), and [Dictus](https://github.com/getdictus/dictus-desktop).

## Author

[VocaHQ](https://github.com/VocaHQ) · [hello@vocahq.com](mailto:hello@vocahq.com)

## License

[AGPL-3.0-or-later](LICENSE). Copyright (C) 2026 Jatin Kumar Malik.
