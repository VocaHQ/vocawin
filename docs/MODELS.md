# VocaWin model adapters

Every engine executes locally. A model is downloaded only once; VocaWin never uploads microphone audio.

## Layout

In-app Download unpacks each model under `%APPDATA%\com.vocahq.vocawin\models` using the catalog ID:

```text
models/
├── whisper-tiny.bin
├── distil-whisper-large-v3.bin
├── voca-hinglish.bin
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
| `voca-hinglish` | whisper.cpp | [q8_0 GGML](https://huggingface.co/Marquestra/Whisper-Hindi2Hinglish-Apex-GGML) of [Oriserve's Hindi2Hinglish Apex](https://huggingface.co/Oriserve/Whisper-Hindi2Hinglish-Apex), pinned to one commit |
| `parakeet-tdt-0.6b-v3` | ONNX Runtime / Parakeet | [int8 archive](https://blob.handy.computer/parakeet-v3-int8.tar.gz) |
| `moonshine-tiny` | ONNX Runtime / Moonshine | [ONNX files](https://huggingface.co/onnx-community/moonshine-tiny-ONNX) |
| `moonshine-base` | ONNX Runtime / Moonshine | [Moonshine base archive](https://blob.handy.computer/moonshine-base.tar.gz) |
| `sensevoice-small` | ONNX Runtime / SenseVoice | [int8 archive](https://blob.handy.computer/sense-voice-int8.tar.gz) |
| `gigaam-v3` | ONNX Runtime / GigaAM | [int8 archive](https://blob.handy.computer/giga-am-v3-int8.tar.gz) |
| `canary-180m` | ONNX Runtime / Canary | [Canary 180M archive](https://blob.handy.computer/canary-180m-flash.tar.gz) |

On Windows, Parakeet and SenseVoice run on ONNX Runtime's DirectML execution provider when DXGI finds a hardware GPU (software/WARP adapters do not count). DirectML uses the system's default adapter, which can differ from the one Settings names for Whisper. Operators DirectML cannot run fall back to CPU inside ONNX Runtime. If a DirectML load or decode fails, VocaWin decodes that take again on CPU. When CPU succeeds, DirectML is blamed and later takes stay on CPU until VocaWin restarts; when CPU fails too, the model or audio is at fault and DirectML stays on. Moonshine, GigaAM and Canary always run on CPU. Canary's int8 decoder runs once per output token; on DirectML each step crosses between CPU and GPU, so it decoded slower than on CPU and sometimes returned nothing, which the CPU fallback cannot catch because it is not an error. Canary is told the chosen language (English, German, Spanish or French); it cannot detect one, and Auto-detect means English.

## Voca Hinglish

The same model as VocaMac's Voca Hinglish: Oriserve's Hindi2Hinglish Apex, a Whisper Large v3 Turbo fine-tune that writes Hindi speech in Roman script. VocaMac runs a WhisperKit (CoreML) build, which Windows cannot load, so VocaWin downloads a whisper.cpp q8_0 conversion made by a third party (Apache-2.0, same license as the model). The URL names a commit, so the file cannot change under the catalog.

It always decodes as English, which is how it was trained to write romanized Hindi, so it ignores the language setting. Its text goes through the text rules as Hindi, so English cleanup does not respell Hindi words. Letters outside Latin and Devanagari are decoder garbage and are removed (`src-tauri/src/hinglish.rs`, matching VocaMac). On silence or room noise the model writes `nan`, so a take that is only `nan` counts as no speech.

## Long takes

Canary, Moonshine and GigaAM get takes longer than 20 seconds in windows of up to about 25 seconds, cut in pauses, and the window texts are joined (`src-tauri/src/chunking.rs`). Decoded whole, Canary skips sentences once real speech runs past roughly 40 seconds, Moonshine repeats a phrase and rejects anything over 64 seconds, and GigaAM's encoder rejects anything over 200 seconds. Whisper already decodes in 30-second windows. Parakeet and SenseVoice decode the whole take.

Parakeet's memory grows with the take: about 1.2 GB for 10 seconds, 1.8 GB for 60 seconds (the default maximum recording) and 4 GB for 300 seconds. Splitting it would bring that down, but Parakeet drops words at the start of each window, so VocaWin keeps the take whole. On a low-RAM PC, keep Parakeet takes short or pick a smaller model.

## Not in the catalog

`parakeet-ctc-1.1b` and `vosk-small-en` stay out of the Models list until an adapter can transcribe them. The UI never offers a Download that cannot run.
