# VocaWin

**Voice-to-text for Windows** | Coming soon

VocaWin is the Windows project in the [Voca](https://vocahq.com/) family. Hold a hotkey, speak, and text is meant to land at your cursor. Speech-to-text is designed to run on this PC after you download a model. There is no public installer yet.

[![Website](https://img.shields.io/badge/Website-vocawin.com-0F6B57?style=flat-square)](https://vocawin.com)
[![VocaHQ](https://img.shields.io/badge/Family-vocahq.com-0F6B57?style=flat-square)](https://vocahq.com)
[![License](https://img.shields.io/badge/License-Coming%20Soon-blue?style=flat-square)](#)
[![Discord](https://img.shields.io/discord/1538633755877580810?style=flat-square&logo=discord&logoColor=white&label=Discord)](https://discord.gg/UMJduhcqn)
[![VocaHQ](https://img.shields.io/badge/VocaHQ-vocahq.com-1a7f4e?style=flat-square)](https://vocahq.com)

---

## What is VocaWin?

VocaWin is being built as native Windows voice typing. After a Whisper model is on disk, recording and speech-to-text are meant to stay on this PC. No Voca account, no hosted speech API.

It sits next to [VocaLinux](https://vocalinux.com), [VocaMac](https://vocamac.com), and [VocaPhone](https://vocaphone.vocahq.com). The family directory is [vocahq.com](https://vocahq.com).

## Key Principles

- **On this PC** - After the model download, transcription is designed to run locally
- **No Voca cloud** - There is no hosted speech service to sign up for
- **Open source** - The repository is public
- **No telemetry in the product** - The planned app does not phone home
- **Windows native** - Tray app, hotkey, WASAPI, text at the caret
- **Honest status** - Coming soon means there is no shipping installer

## Planned Features

- **System-wide text injection** - Transcribed text appears wherever your cursor is
- **Push-to-talk and toggle** - Hold a hotkey (planned default: Right Ctrl) or double-tap to toggle
- **GPU acceleration** - NVIDIA CUDA, AMD, and Intel paths via whisper.cpp
- **Language support** - Follows the selected Whisper model
- **Configurable settings** - Hotkeys, models, languages, silence detection
- **Visual feedback** - Tray icon states for idle, recording, and processing
- **Clipboard preservation** - Save and restore the clipboard after injection

## The Voca ecosystem

Same privacy bar, different machines. Start at [vocahq.com](https://vocahq.com) for the map.

| Platform | Project | Website | GitHub | Status |
|----------|---------|---------|--------|--------|
| Family | **VocaHQ** | [vocahq.com](https://vocahq.com) | [VocaHQ](https://github.com/VocaHQ) | Directory |
| Linux | **VocaLinux** | [vocalinux.com](https://vocalinux.com) | [VocaHQ/vocalinux](https://github.com/VocaHQ/vocalinux) | Available |
| macOS | **VocaMac** | [vocamac.com](https://vocamac.com) | [VocaHQ/vocamac](https://github.com/VocaHQ/vocamac) | Beta |
| iPhone / Android | **VocaPhone** | [vocaphone.vocahq.com](https://vocaphone.vocahq.com) | [VocaHQ/vocaphone](https://github.com/VocaHQ/vocaphone) | Beta / source build |
| Windows | **VocaWin** | [vocawin.com](https://vocawin.com) | [VocaHQ/vocawin](https://github.com/VocaHQ/vocawin) | Coming soon |
| Infrastructure | **VocaGateway** | | [VocaHQ/vocagateway](https://github.com/VocaHQ/vocagateway) | Early |

VocaGateway is optional self-hosted compute for other Voca clients. VocaWin does not expose a gateway mode today.

## Tech Stack (Planned)

- **Speech Engine**: [whisper.cpp](https://github.com/ggerganov/whisper.cpp) with GPU acceleration
- **Platform**: Windows 10/11
- **GPU Support**: NVIDIA CUDA, AMD, Intel
- **Languages**: determined by the downloaded Whisper model

## System Requirements (Expected)

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

VocaWin is in early development. Star the repo to follow progress, or browse the family at [vocahq.com](https://vocahq.com).

## Author

[VocaHQ](https://github.com/VocaHQ) · [hello@vocahq.com](mailto:hello@vocahq.com)

## License

Coming soon. See [VocaMac (AGPL-3.0)](https://github.com/VocaHQ/vocamac/blob/main/LICENSE) and [VocaLinux](https://github.com/VocaHQ/vocalinux/blob/main/LICENSE) for reference.
