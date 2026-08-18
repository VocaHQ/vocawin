#!/usr/bin/env python3
"""Write the dictation start/stop WAVs under src-tauri/sounds/. Stdlib only."""

from __future__ import annotations

import math
import struct
import wave
from pathlib import Path

SAMPLE_RATE = 22050
ROOT = Path(__file__).resolve().parents[1] / "src-tauri" / "sounds"


def clamp(value: float, low: float = -1.0, high: float = 1.0) -> float:
    return low if value < low else high if value > high else value


def smoothstep(t: float) -> float:
    t = 0.0 if t < 0.0 else 1.0 if t > 1.0 else t
    return t * t * (3.0 - 2.0 * t)


def sine_sq_env(t: float, duration: float) -> float:
    if duration <= 0.0:
        return 0.0
    x = t / duration
    if x <= 0.0 or x >= 1.0:
        return 0.0
    s = math.sin(math.pi * x)
    return s * s


def write_wav(path: Path, samples: list[float]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "w") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(SAMPLE_RATE)
        frames = b"".join(
            struct.pack("<h", int(clamp(sample) * 32767.0)) for sample in samples
        )
        wav.writeframes(frames)


def render(duration: float, fn) -> list[float]:
    count = max(1, int(round(duration * SAMPLE_RATE)))
    samples = []
    phase = 0.0
    for index in range(count):
        t = index / SAMPLE_RATE
        freq, amp = fn(t, duration)
        phase += freq * math.tau / SAMPLE_RATE
        samples.append(math.sin(phase) * amp)
    return samples


def glide(duration: float, start_hz: float, end_hz: float, amp: float) -> list[float]:
    def frame(t: float, dur: float) -> tuple[float, float]:
        mix = smoothstep(t / dur)
        return start_hz + (end_hz - start_hz) * mix, amp * sine_sq_env(t, dur)

    return render(duration, frame)


def ticks(notes: list[float], tick: float, gap: float, amp: float) -> list[float]:
    out: list[float] = []
    for i, hz in enumerate(notes):
        out.extend(glide(tick, hz, hz, amp))
        if i + 1 < len(notes):
            out.extend([0.0] * int(round(gap * SAMPLE_RATE)))
    return out


def swell_glide(duration: float, start_hz: float, end_hz: float, amp: float) -> list[float]:
    def frame(t: float, dur: float) -> tuple[float, float]:
        mix = smoothstep(t / dur)
        # Late peak so the fifth opens as it rises.
        env = sine_sq_env(t, dur) * (0.45 + 0.55 * mix)
        return start_hz + (end_hz - start_hz) * mix, amp * env

    return render(duration, frame)


def chirp(duration: float, low: float, peak: float, settle: float, amp: float) -> list[float]:
    def frame(t: float, dur: float) -> tuple[float, float]:
        x = t / dur
        if x < 0.45:
            mix = smoothstep(x / 0.45)
            hz = low + (peak - low) * mix
        else:
            mix = smoothstep((x - 0.45) / 0.55)
            hz = peak + (settle - peak) * mix
        return hz, amp * sine_sq_env(t, dur)

    return render(duration, frame)


def ping(duration: float, hz: float, amp: float) -> list[float]:
    samples = []
    for index in range(int(round(duration * SAMPLE_RATE))):
        t = index / SAMPLE_RATE
        decay = math.exp(-t * 28.0)
        attack = min(1.0, t * 400.0)
        samples.append(math.sin(t * hz * math.tau) * amp * decay * attack)
    return samples


def scale_notes(notes: list[float], note: float, amp: float) -> list[float]:
    out: list[float] = []
    for hz in notes:
        out.extend(glide(note, hz, hz * 1.01, amp))
    return out


TONES = {
    "lift": (
        glide(0.55, 349.23, 440.00, 0.16),
        glide(0.55, 440.00, 349.23, 0.16),
    ),
    "flick": (
        glide(0.22, 349.23, 440.00, 0.16),
        glide(0.22, 440.00, 349.23, 0.16),
    ),
    "ember": (
        glide(0.65, 196.00, 261.63, 0.14),
        glide(0.65, 261.63, 196.00, 0.14),
    ),
    "step": (
        ticks([261.63, 329.63], 0.07, 0.045, 0.10),
        ticks([329.63, 261.63], 0.07, 0.045, 0.10),
    ),
    "voca": (
        swell_glide(0.48, 261.63, 392.00, 0.15),
        swell_glide(0.48, 392.00, 261.63, 0.15),
    ),
    "soft": (
        glide(0.07, 261.63, 261.63, 0.06),
        glide(0.07, 196.00, 196.00, 0.055),
    ),
    "chirp": (
        chirp(0.20, 1320.0, 1980.0, 1540.0, 0.11),
        chirp(0.20, 1540.0, 1100.0, 1240.0, 0.10),
    ),
    "scale": (
        scale_notes([440.00, 523.25], 0.09, 0.12),
        scale_notes([523.25, 440.00], 0.09, 0.12),
    ),
    "drop": (
        swell_glide(0.52, 146.83, 196.00, 0.14),
        swell_glide(0.62, 174.61, 110.00, 0.15),
    ),
    "glass": (
        ping(0.18, 2093.0, 0.09),
        ping(0.20, 1760.0, 0.085),
    ),
}


def main() -> None:
    for theme, (start, stop) in TONES.items():
        write_wav(ROOT / theme / "start.wav", start)
        write_wav(ROOT / theme / "stop.wav", stop)
        print(f"wrote {theme}")


if __name__ == "__main__":
    main()
