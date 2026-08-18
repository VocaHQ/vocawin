# VocaWin model adapters

Every engine executes locally. A model is downloaded only once; VocaWin never uploads microphone audio.

## Layout

Place each extracted ONNX model directory under `%APPDATA%\com.vocahq.vocawin\models` using the catalog ID as the directory name:

```text
models/
├── parakeet-tdt-0.6b-v3/
├── moonshine-tiny/
├── moonshine-base/
├── sensevoice-small/
├── gigaam-v3/
└── canary-180m/
```

The adapter validates that the directory exists, then lets `transcribe-rs` validate the engine-specific ONNX files. This avoids accepting incomplete downloads as installed models.

## Supported adapters

| VocaWin ID | Adapter | Source model |
| --- | --- | --- |
| `parakeet-tdt-0.6b-v3` | ONNX Runtime / Parakeet | [int8 ONNX](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/tree/main) |
| `moonshine-tiny` | ONNX Runtime / Moonshine | [Moonshine ONNX releases](https://github.com/usefulsensors/moonshine) |
| `moonshine-base` | ONNX Runtime / Moonshine | [Moonshine base archive](https://blob.handy.computer/moonshine-base.tar.gz) |
| `sensevoice-small` | ONNX Runtime / SenseVoice | [sherpa-onnx release](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models) |
| `gigaam-v3` | ONNX Runtime / GigaAM | [GigaAM v3 ONNX](https://huggingface.co/istupakov/gigaam-v3-onnx/tree/main) |
| `canary-180m` | ONNX Runtime / Canary | [Canary 180M ONNX](https://huggingface.co/istupakov/canary-180m-flash-onnx) |

VocaWin's Windows dependency enables ONNX Runtime's DirectML execution provider. If it is not available for a model or GPU, ONNX Runtime falls back to CPU.

## Not yet wired

`parakeet-ctc-1.1b` and `vosk-small-en` remain visible in the catalog but do not yet have a production adapter. They fail with a clear local error rather than falling back to any network API.
