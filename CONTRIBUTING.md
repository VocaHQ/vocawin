# Contributing to VocaWin

Thanks for helping make voice-to-text private and free. This project
follows a few simple conventions to keep the codebase reviewable and
the build reproducible.

## Development environment

- Windows 10 1809+ (x64) is the primary target.
- C++20 compiler: MSVC 19.30+ or MinGW-w64 g++ 13+.
- CMake 3.21+, Ninja, Python 3.10+.
- No vcpkg/Conan — dependencies are vendored via `FetchContent` or
  inlined as headers.

## Code style

- **C++20**, no extensions. Set `CMAKE_CXX_EXTENSIONS OFF`.
- **Strict types** — no `as any`/`@ts-ignore` equivalents. Use
  `std::optional`, `std::variant`, `enum class` for sums.
- **Pure functions preferred** where the surface allows. Test them
  in isolation.
- **No `using namespace std;` in headers.** Allowed in `.cpp` files
  inside the namespace.
- **Headers are `#pragma once` only.** No include guards.
- **Comments are scarce and explain the non-obvious.** A short
  comment naming the scenario or constraint is fine; a paragraph
  describing what the code obviously does is not.

## Testing

- Tests live in `tests/`, one file per module, named `test_<module>.cpp`.
- Use `<cassert>` — no gtest, no Catch2. Keep the framework
  dependency-free so the test binary is one .cpp + one .exe.
- TDD is mandatory: write a failing test, watch it fail, then make it
  pass. Capture both RED and GREEN output in your commit message.
- Run the full suite: `ctest --test-dir build/debug --output-on-failure`
- Coverage gate: `python scripts/check_coverage.py build/debug` must
  pass at ≥ 80%.

## Build presets

| Preset | Purpose |
|---|---|
| `debug` | CPU only, tests on, sanitizers off |
| `release` | Optimized CPU build |
| `release-cuda` | NVIDIA CUDA backend |
| `release-vulkan` | Vulkan backend (AMD/Intel/NVIDIA) |

## Pull request checklist

- [ ] All 23+ tests pass
- [ ] Coverage gate ≥ 80%
- [ ] No new compiler warnings (`-Wall -Wextra`)
- [ ] `docs/SPEC.md` updated if you changed a public surface
- [ ] `README.md` and `CHANGELOG.md` updated for user-facing changes
- [ ] Atomic commits with conventional commit messages

## Reporting bugs

Open an issue with:
- Repro steps (hotkey used, model loaded, app focused)
- The relevant `vocawin.log` excerpt from `%LOCALAPPDATA%\VocaWin\logs\`
- Windows version and CPU/RAM
