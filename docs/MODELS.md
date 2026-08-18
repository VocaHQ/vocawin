# VocaWin model adapters

Every engine executes locally. A model is downloaded only once; VocaWin never uploads microphone audio.

## Layout

In-app Download unpacks each model under `%APPDATA%\com.vocahq.vocawin\models` using the catalog ID:

```text
models/
├── whisper-tiny.bin
├── distil-whisper-large-v3.bin
├── parakeet-tdt-0.6b-v3/
├── moonshine-tiny/
├── moonshine-base/
├── sensevoice-small/
├── gigaam-v3/
└── canary-180m/
```

Whisper-family models are a single GGML `.bin`. ONNX models are directories whose filenames match what `transcribe-rs` expects.

## Supported adapters

| VocaWin ID | Adapter | Source package |
| --- | --- | --- |
| `whisper-*` / `distil-whisper-large-v3` | whisper.cpp | Official GGML `.bin` from Hugging Face |
| `parakeet-tdt-0.6b-v3` | ONNX Runtime / Parakeet | [int8 archive](https://blob.handy.computer/parakeet-v3-int8.tar.gz) |
| `moonshine-tiny` | ONNX Runtime / Moonshine | [ONNX files](https://huggingface.co/onnx-community/moonshine-tiny-ONNX) |
| `moonshine-base` | ONNX Runtime / Moonshine | [Moonshine base archive](https://blob.handy.computer/moonshine-base.tar.gz) |
| `sensevoice-small` | ONNX Runtime / SenseVoice | [int8 archive](https://blob.handy.computer/sense-voice-int8.tar.gz) |
| `gigaam-v3` | ONNX Runtime / GigaAM | [int8 archive](https://blob.handy.computer/giga-am-v3-int8.tar.gz) |
| `canary-180m` | ONNX Runtime / Canary | [Canary 180M archive](https://blob.handy.computer/canary-180m-flash.tar.gz) |

VocaWin's Windows dependency enables ONNX Runtime's DirectML execution provider. If it is not available for a model or GPU, ONNX Runtime falls back to CPU.

## Not in the catalog

`parakeet-ctc-1.1b` and `vosk-small-en` stay out of the Models list until an adapter can transcribe them. The UI never offers a Download that cannot run.
