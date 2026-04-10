# VocaWin

**100% Offline Voice-to-Text for Windows** | Coming Soon 🚀

> Your voice. Your PC. Your privacy. No data leaves your system.

[![Website](https://img.shields.io/badge/Website-vocawin.com-0078D4?style=flat-square)](https://vocawin.com)
[![License](https://img.shields.io/badge/License-Coming%20Soon-blue?style=flat-square)](#)

---

## What is VocaWin?

VocaWin brings **100% offline, privacy-first voice-to-text** to Windows. Hold a hotkey, speak, and text appears at your cursor. In any app. No cloud, no accounts, no subscriptions.

VocaWin is the Windows counterpart to [VocaMac](https://vocamac.com) and [VocaLinux](https://vocalinux.com), completing the Voca ecosystem across all major desktop platforms.

## Key Principles

- **100% Offline** - All speech recognition happens locally on your PC
- **Privacy First** - Your voice data never leaves your computer
- **No Data Leaves Your System** - Zero network calls for transcription
- **Open Source** - Every line of code is public and auditable
- **No Telemetry** - No analytics, no tracking, no crash reporting
- **Free Forever** - No subscriptions, no accounts, no premium tiers
- **Windows Native** - Built for Windows with GPU acceleration support
- **Zero Cloud Dependencies** - Works without an internet connection

## Planned Features

- **System-Wide Text Injection** - Transcribed text appears wherever your cursor is. Browsers, Slack, VS Code, Word, Excel, terminals. Everywhere.
- **Push-to-Talk & Toggle Mode** - Hold a hotkey to record or double-tap to toggle. Simple, predictable control.
- **GPU Accelerated** - NVIDIA CUDA, AMD, and Intel GPU acceleration for blazing fast transcription.
- **99+ Languages** - Auto-detect or specify your language with Whisper model support.
- **Smart Model Selection** - Auto-detects your hardware and recommends the optimal model.
- **Silence Detection** - Auto-stops recording after you stop speaking with adjustable sensitivity.
- **Fully Configurable** - Choose hotkeys, models, languages, and silence detection thresholds.
- **Visual Feedback** - System tray icon changes and audio level indicators show recording state.
- **Clipboard Preservation** - Your clipboard is saved and restored after text injection.

## The Voca Ecosystem

VocaWin is part of a family of privacy-first, offline voice dictation tools. Same mission, every operating system.

| Platform | Project | Website | GitHub | Status |
|----------|---------|---------|--------|--------|
| 🍎 macOS | **VocaMac** | [vocamac.com](https://vocamac.com) | [jatinkrmalik/vocamac](https://github.com/jatinkrmalik/vocamac) | Beta v0.3.0 |
| 🐧 Linux | **VocaLinux** | [vocalinux.com](https://vocalinux.com) | [jatinkrmalik/vocalinux](https://github.com/jatinkrmalik/vocalinux) | Beta v0.8.0 |
| 🖥️ Windows | **VocaWin** | [vocawin.com](https://vocawin.com) | [jatinkrmalik/vocawin](https://github.com/jatinkrmalik/vocawin) | Coming Soon |

## Tech Stack (Current Direction)

- **Language**: C++ (native Windows app)
- **UI Framework**: WinUI 3 (Windows App SDK)
- **Speech Engine**: [whisper.cpp](https://github.com/ggerganov/whisper.cpp)
- **Audio Input**: WASAPI (Windows Audio Session API)
- **Text Injection**: Windows `SendInput` (with clipboard-preserving fallback)
- **GPU Backends**:
  - Phase 1/2: CUDA + Vulkan
  - Planned follow-up: DirectML backend path
- **Platform**: Windows 10/11
- **Languages**: 99+ via Whisper models

## System Requirements (Expected)

- Windows 10 or later
- 4 GB RAM (8 GB+ recommended for larger models)
- Microphone
- GPU recommended for faster transcription (NVIDIA, AMD, or Intel)

## Windows Development Environment

Because VocaWin is a native Windows application, real feature validation must be done on Windows (hotkeys, tray behavior, WASAPI audio capture, and text injection).

Recommended setup:

- Windows 11 machine or VM (Parallels/UTM acceptable for early dev)
- Visual Studio 2022 with Desktop C++ workload
- Windows SDK (10/11)
- CMake + Ninja
- vcpkg
- LLVM tools (for coverage)
- WiX Toolset v4 (for MSI packaging in later phases)

Note: macOS can still be used for docs/planning and generic C++ work, but Windows is the source of truth for runtime behavior.

## Website

The landing page at [vocawin.com](https://vocawin.com) is hosted via GitHub Pages.

### Deploying to GitHub Pages

1. Push this repo to GitHub
2. Go to **Settings > Pages**
3. Set source to `main` branch, root `/`
4. The `CNAME` file maps the custom domain `vocawin.com`
5. Configure your DNS to point `vocawin.com` to GitHub Pages

## Development Status

Phase 1 foundation scaffolding is implemented in this repository:

- C++ project layout with `src/` modules for app, config, util, and UI
- Root `CMakeLists.txt` and `CMakePresets.json`
- Settings and logging scaffolding
- Single-instance guard scaffolding
- Basic tray icon service stub
- Multi-target tests under `tests/`
- Coverage gate script: `scripts/check_coverage.sh` (80%+ line coverage target)

CI now runs real build and test steps for the current foundation, including coverage enforcement.

See `docs/SPEC.md` for the complete engineering and product spec.

## Contributing

VocaWin is in early development. Stay tuned for contribution guidelines. In the meantime, star the repo to follow progress!

## Author

Made with ❤️ by [Jatin K Malik](https://x.com/intent/user?screen_name=jatinkrmalik)

## License

Coming soon. See [VocaMac (AGPL-3.0)](https://github.com/jatinkrmalik/vocamac/blob/main/LICENSE) and [VocaLinux (GPL-3.0)](https://github.com/jatinkrmalik/vocalinux/blob/main/LICENSE) for reference.
