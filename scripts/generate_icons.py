"""Generate placeholder tray icons for VocaWin.

Produces 32x32 32-bit RGBA PNGs (interoperable with ICO conversion):
  - tray-idle.png       : blue circle
  - tray-recording.png  : red circle
  - tray-processing.png : purple circle
  - tray-error.png      : orange circle
  - vocawin.png         : blue square with white "V"

For Windows, the build also produces .ico files (32x32, 32-bit BGRA)
from the PNGs via a small ICO writer.

Usage: python scripts/generate_icons.py [out_dir]
"""

from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path

SIZE = 32
COLORS = {
    "tray-idle":       (0x00, 0x78, 0xD4, 0xFF),
    "tray-recording":  (0xE8, 0x11, 0x23, 0xFF),
    "tray-processing": (0x88, 0x6C, 0xE4, 0xFF),
    "tray-error":      (0xFF, 0xB9, 0x00, 0xFF),
    "vocawin":         (0x00, 0x78, 0xD4, 0xFF),
}


def _write_png(path: Path, pixels: bytes) -> None:
    """Write a 32x32 32-bit RGBA PNG (no filter)."""
    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)  # filter type none
        raw.extend(pixels[y * SIZE * 4:(y + 1) * SIZE * 4])
    idat = zlib.compress(bytes(raw), 9)
    path.write_bytes(sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) +
                     chunk(b"IEND", b""))


def _write_ico(path: Path, png_path: Path) -> None:
    """Single-image .ico embedding a 32x32 PNG (Vista+ format)."""
    data = png_path.read_bytes()
    png_size = len(data)
    # ICONDIR
    out = struct.pack("<HHH", 0, 1, 1)
    # ICONDIRENTRY: width=32, height=32, colors=0, reserved=0, planes=1,
    # bpp=32, sizeInBytes=len(png), offset=22
    out += struct.pack("<BBBBHHII", SIZE, SIZE, 0, 0, 1, 32, png_size, 22)
    out += data
    path.write_bytes(out)


def _circle_pixels(rgba: tuple[int, int, int, int],
                   cx: float = SIZE / 2, cy: float = SIZE / 2,
                   r: float = SIZE / 2 - 1.5) -> bytes:
    out = bytearray()
    for y in range(SIZE):
        for x in range(SIZE):
            dx = x - cx + 0.5
            dy = y - cy + 0.5
            d = (dx * dx + dy * dy) ** 0.5
            if d <= r:
                out.extend(rgba)
            else:
                out.extend((0, 0, 0, 0))
    return bytes(out)


def _vo_pixels(base: tuple[int, int, int, int]) -> bytes:
    """Blue square base with a white "V" shape carved in."""
    out = bytearray()
    white = (0xFF, 0xFF, 0xFF, 0xFF)
    for y in range(SIZE):
        for x in range(SIZE):
            # V glyph: 3 lines from (8,6)-(16,26)-(24,6)
            in_v = False
            for t_step in range(0, 8):
                t = t_step / 7.0
                px = 8 + (24 - 8) * t
                py = 6 + (26 - 6) * t
                if abs(x - px) < 2.0 and abs(y - py) < 2.0:
                    in_v = True
                    break
            if in_v:
                out.extend(white)
            else:
                out.extend(base)
    return bytes(out)


def main(argv: list[str]) -> int:
    out_dir = Path(argv[1]) if len(argv) > 1 else Path("resources/icons")
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, color in COLORS.items():
        if name == "vocawin":
            pixels = _vo_pixels(color)
        else:
            pixels = _circle_pixels(color)
        png_path = out_dir / f"{name}.png"
        ico_path = out_dir / f"{name}.ico"
        _write_png(png_path, pixels)
        _write_ico(ico_path, png_path)
        print(f"  {name:22s} png={png_path.stat().st_size:6d}  "
              f"ico={ico_path.stat().st_size:6d}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
