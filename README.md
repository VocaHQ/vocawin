<p align="center">
  <img src="web/assets/brand/voca-logo-512.png" alt="VocaWin" width="128" height="128">
</p>

<h1 align="center">VocaWin</h1>

<p align="center"><strong>Voice-to-text for Windows.</strong></p>

<div align="center">

[![Release](https://img.shields.io/github/v/release/VocaHQ/vocawin?include_prereleases&sort=semver)](https://github.com/VocaHQ/vocawin/releases)
[![Nightly](https://img.shields.io/badge/Nightly-download-blueviolet)](https://github.com/VocaHQ/vocawin/releases/tag/nightly)
[![Status](https://img.shields.io/badge/status-beta-yellow)](#development)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey)](#system-requirements)
[![Privacy](https://img.shields.io/badge/privacy-on%20this%20PC-success)](#key-principles)

[![License](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Website](https://img.shields.io/badge/Website-vocawin.com-informational)](https://vocawin.com)
[![Discord](https://img.shields.io/discord/1538633755877580810?logo=discord&logoColor=white&label=Discord)](https://discord.gg/t6muquAJbm)
[![Follow on X](https://img.shields.io/badge/Follow%20%40vocahq-000000?style=flat&logo=x&logoColor=white)](https://x.com/vocahq)
[![VocaHQ](https://img.shields.io/badge/VocaHQ-vocahq.com-1a7f4e)](https://vocahq.com)

Beta. Unsigned. Testers can grab a build from [GitHub Releases](https://github.com/VocaHQ/vocawin/releases). This is not a signed store build and not a stable public release.

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
- **Honest status** - Beta. Unsigned. Windows will likely say the publisher is unknown.

## Try it

Testers can install an unsigned beta today. Download the NSIS `.exe` or the MSI from [GitHub Releases](https://github.com/VocaHQ/vocawin/releases). Windows will likely say the publisher is unknown. That is SmartScreen. More info, then Run anyway if you trust the file. Read [the setup guide](docs/setup.md) first.

Want today's `main` instead of the last tagged Release? Use the [nightly](https://github.com/VocaHQ/vocawin/releases/tag/nightly). Same unsigned NSIS and MSI, rebuilt when app source on `main` changes. Prefer the latest tagged Release if you want the last cut we named.

This is a tester build you can run today. It is not a store listing and not a stable public release.

### What works

- **Hold a hotkey, speak, text at the caret** - Default is Right Alt, the same hold as VocaLinux. Double-tap toggles. You can change the hotkey in Settings.
- **Tray** - Idle, recording, and processing icon states. Close goes to the tray. Show window / Quit.
- **Settings** - Hotkeys, models, languages, silence detection, sounds, start on login.
- **Local models** - In-app Download for Whisper/whisper.cpp, Distil-Whisper, Parakeet, Moonshine, SenseVoice, GigaAM, and Canary.
- **GPU** - whisper.cpp on Vulkan with CPU fallback. ONNX Runtime on DirectML with CPU fallback.
- **Clipboard stays yours.** Insertion types at the caret and does not replace what you copied. Turn on Copy to clipboard in Settings if you want the transcript left there. Clipboard paste is only a fallback, and that path restores the previous clipboard.

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
| iPhone / Android | **VocaPhone** | [vocaphone.vocahq.com](https://vocaphone.vocahq.com) | [VocaHQ/vocaphone](https://github.com/VocaHQ/vocaphone) | Android beta / iOS TestFlight |
| Windows | **VocaWin** | [vocawin.com](https://vocawin.com) | [VocaHQ/vocawin](https://github.com/VocaHQ/vocawin) | Beta |
| Infrastructure | **VocaGateway** | [vocagateway.vocahq.com](https://vocagateway.vocahq.com) | [VocaHQ/vocagateway](https://github.com/VocaHQ/vocagateway) | Early |

VocaGateway is optional self-hosted compute for other Voca clients. VocaWin does not expose a gateway mode today.

## Tech Stack

- **Speech Engine**: [whisper.cpp](https://github.com/ggerganov/whisper.cpp) and ONNX Runtime, both local
- **Platform**: Windows 10/11
- **GPU**: Vulkan for Whisper, DirectML for ONNX, CPU fallback for both
- **Languages**: determined by the downloaded model

## Development

### Prerequisites

A macOS or Linux host can validate the frontend and the Rust command layer (`npm run check`, `cargo test`). A real desktop build (`npm run tauri dev` / `npm run tauri build`) needs Windows 10/11 plus the same toolchain CI installs:

- Node.js 20+ (CI uses 22)
- Rust stable
- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/): Microsoft C++ Build Tools (MSVC) and WebView2
- [LLVM](https://github.com/llvm/llvm-project/releases) Windows installer (`LLVM-*-win64.exe`), so `libclang` is on disk. CI sets `LIBCLANG_PATH` to `C:\Program Files\LLVM\bin`.
- [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home) for whisper.cpp Vulkan. Confirm `VULKAN_SDK` points at the SDK root (CI uses `C:\VulkanSDK\1.3.290.0`).
- [CMake](https://cmake.org/download/) for the whisper/ggml native build. Add it to PATH. CI also installs [Ninja](https://ninja-build.org/) and sets `CMAKE_GENERATOR=Ninja`.
- A way around Windows `MAX_PATH` (260 characters). whisper.cpp Vulkan shader nests go deep even if the repo sits at `C:\vocawin`. Enable [OS long paths](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation), or use a short `CARGO_TARGET_DIR` the way CI does (`C:\t`).

Open a new terminal after installing so PATH and env vars refresh. Chocolatey (`choco install cmake llvm ninja`) matches the CI installers if you already use it.

### Commands

```bash
npm install
npm run tauri dev       # desktop development
npm run tauri build     # NSIS .exe on Windows (MSI is paused while the version is X.Y.Z-beta)
npm run check           # TypeScript build + Rust tests
```

Windows injection, WASAPI, Vulkan, DirectML, and installer artifacts must be exercised on Windows 10/11.

### Windows build troubleshooting

These are the failures that show up when a CI dependency is missing locally.

**Unable to find libclang** (`couldn't find any valid shared libraries matching ['clang.dll', 'libclang.dll']`)

Install LLVM from the [Windows installer](https://github.com/llvm/llvm-project/releases). Then, if the crate still cannot see it:

```powershell
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
```

Set that as a user environment variable so it survives a reboot. Confirm `clang.dll` or `libclang.dll` is in that folder.

**Please install Vulkan SDK and ensure that VULKAN_SDK env variable is set**

Install the [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home) and reopen the terminal. Check:

```powershell
echo $env:VULKAN_SDK
```

If it is empty, point it at the SDK root, for example `C:\VulkanSDK\1.3.290.0` (use the version you installed). CI pins `1.3.290.0`. A current LunarG Windows SDK is fine for local work.

**is cmake not installed?** (`failed to execute command: program not found`)

Install [CMake](https://cmake.org/download/) and tick "Add CMake to the system PATH" (or `winget install Kitware.CMake`). Confirm `cmake --version` in a new shell. The whisper/ggml crate build needs it.

**exceeds the OS max path limit** / **The fully qualified file name must be less than 260 characters**

whisper-rs-sys nests `target\debug\build\whisper-rs-sys-*\out\build\ggml\ggml-vulkan\...` deep enough to hit `MAX_PATH`. Either of these is enough:

1. Point Cargo at a short target dir, matching CI:

```powershell
$env:CARGO_TARGET_DIR = "C:\t"
$env:CMAKE_GENERATOR = "Ninja"   # optional; CI uses Ninja to avoid MSBuild path races
```

Put [Ninja](https://ninja-build.org/) on PATH if you set `CMAKE_GENERATOR=Ninja`.

2. Enable Windows long paths, then reboot:
   - Group Policy: Computer Configuration, Administrative Templates, System, Filesystem, Enable Win32 long paths
   - Or registry: `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem`, DWORD `LongPathsEnabled` = `1`

Windows CI uses `CARGO_TARGET_DIR=C:\t` and `CMAKE_GENERATOR=Ninja`. It does not enable OS long paths on the runner.

### Installers and CI

Windows CI builds an unsigned NSIS installer on pushes to `main` and on `workflow_dispatch`, then uploads it as a GitHub Actions artifact. Pull requests only run `cargo test`, so a docs change does not package the setup wizard. The installer stays unsigned. SmartScreen can still warn.

Pushing a `v*` tag builds the same NSIS installer and attaches it to a GitHub Release. While the app version is `X.Y.Z-beta`, tagged cuts are NSIS only (MSI is paused because WiX rejects the `-beta` marker). Testers should use [Releases](https://github.com/VocaHQ/vocawin/releases), not the workflow artifact. The build is unsigned, not a purchased CA or store signature. Windows will likely still warn. More info, then Run anyway. There is no Microsoft Store listing and no auto-update. Read [the setup guide](docs/setup.md) before you install, and [file an issue](https://github.com/VocaHQ/vocawin/issues) if something breaks. [vocawin.com](https://vocawin.com) points at the same download.

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

VocaWin is in early development. Download the unsigned beta from [Releases](https://github.com/VocaHQ/vocawin/releases) if you want to try it. File bugs on [Issues](https://github.com/VocaHQ/vocawin/issues). The family directory is [vocahq.com](https://vocahq.com).

## Project references

The product design and model support are informed by [VocaMac](https://github.com/VocaHQ/vocamac), [VocaLinux](https://github.com/VocaHQ/vocalinux), [VocaPhone](https://github.com/VocaHQ/vocaphone), [VocaGateway](https://github.com/VocaHQ/vocagateway), [Handy](https://github.com/cjpais/Handy), and [Dictus](https://github.com/getdictus/dictus-desktop).

## Author

[VocaHQ](https://github.com/VocaHQ) · [hello@vocahq.com](mailto:hello@vocahq.com)

## License

[AGPL-3.0-or-later](LICENSE). Copyright (C) 2026 Jatin Kumar Malik.
