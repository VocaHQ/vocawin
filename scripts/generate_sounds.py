"""Generate WAV feedback sounds for VocaWin.

Produces 16-bit PCM mono WAV files at 44.1 kHz:
  - start.wav   : rising 880 Hz -> 1320 Hz chirp, 80 ms
  - stop.wav    : falling 1320 Hz -> 880 Hz chirp, 80 ms
  - error.wav   : square 220 Hz pulse, 250 ms

Usage: python scripts/generate_sounds.py [out_dir]
"""

from __future__ import annotations

import math
import os
import struct
import sys
import wave
from pathlib import Path

SAMPLE_RATE = 44100
AMPLITUDE = 0.4  # peak amplitude (0..1)


def _s16le_samples(samples: list[float]) -> bytes:
    out = bytearray()
    for s in samples:
        v = max(-1.0, min(1.0, s))
        out += struct.pack("<h", int(v * 32767))
    return bytes(out)


def _envelope(i: int, n: int) -> float:
    # 5 ms attack, 5 ms release
    a = int(0.005 * SAMPLE_RATE)
    if i < a:
        return i / a
    if i > n - a:
        return (n - i) / a
    return 1.0


def chirp(f0: float, f1: float, duration_s: float) -> list[float]:
    n = int(duration_s * SAMPLE_RATE)
    out = []
    for i in range(n):
        t = i / SAMPLE_RATE
        f = f0 + (f1 - f0) * (i / n)
        env = _envelope(i, n)
        out.append(AMPLITUDE * env * math.sin(2 * math.pi * f * t))
    return out


def pulse(freq: float, duration_s: float) -> list[float]:
    n = int(duration_s * SAMPLE_RATE)
    out = []
    period = int(SAMPLE_RATE / freq)
    for i in range(n):
        t = i / SAMPLE_RATE
        env = _envelope(i, n)
        square = 1.0 if (i % period) < (period // 2) else -1.0
        out.append(AMPLITUDE * env * square)
    return out


def write_wav(path: Path, samples: list[float]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        w.writeframes(_s16le_samples(samples))


def main(argv: list[str]) -> int:
    out_dir = Path(argv[1]) if len(argv) > 1 else Path("resources/sounds")
    write_wav(out_dir / "start.wav", chirp(880.0, 1320.0, 0.08))
    write_wav(out_dir / "stop.wav", chirp(1320.0, 880.0, 0.08))
    write_wav(out_dir / "error.wav", pulse(220.0, 0.25))
    for name in ("start.wav", "stop.wav", "error.wav"):
        size = (out_dir / name).stat().st_size
        print(f"  {name:12s} {size:6d} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
