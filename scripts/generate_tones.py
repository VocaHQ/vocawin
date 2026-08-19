#!/usr/bin/env python3
"""The start/stop WAVs in src-tauri/sounds/ are the preview files.

Do not synthesize replacements. The committed bytes are the pairs from the
preview page (default Voca is 05-fifth). Running this used to overwrite them
with a remake that did not match.
"""

from __future__ import annotations

import sys


def main() -> int:
    print(
        "src-tauri/sounds/*.wav are the preview files. "
        "This script will not overwrite them.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
