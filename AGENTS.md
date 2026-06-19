# AGENTS.md — AI coding guidelines

This file is the contract between VocaWin maintainers and AI coding
agents. It exists so that an agent (or a human) can pick up any task
in this repository and execute it without re-deriving the project's
conventions.

## Project snapshot

- **Language**: C++20, no extensions.
- **Build**: CMake 3.21+ presets (`debug`, `release`, `release-cuda`,
  `release-vulkan`).
- **Tests**: `<cassert>` only, 23 binaries, one per module.
- **Coverage gate**: `python scripts/check_coverage.py build/debug`
  must report ≥ 80%.
- **No gtest / no Catch2 / no vcpkg.** Test binaries must link with
  nothing but `vocawin_core` (static lib).
- **Platform**: Windows 10 1809+ x64 is the primary target. Linux/macOS
  is supported for non-Win32 modules (audio, model manager, settings
  store, updater) so the codebase is portable.

## Non-negotiable rules

1. **No `as any` / `// @ts-ignore` / unchecked casts.** Use
   `std::optional`, `std::variant`, `enum class`, or a real parser.
2. **No mocks, no fakes, no "demo" data** in production code. Test
   fixtures use real files in `build/test-*/`; the production code
   path takes real inputs.
3. **No silent scope reduction.** If a feature is too big for one
   PR, split the work, but each PR must be a working slice — never
   "X is now a stub, will fill in later."
4. **Strict types and exhaustive matching.** If a switch covers an
   enum, the compiler must reject a missing case. `default:` is
   forbidden for enums.
5. **All public functions are documented in the header**, even
   briefly. A one-line comment naming the contract is enough; a
   paragraph is too much.
6. **Test first.** RED → GREEN → REFACTOR. Do not edit production
   code without a failing test in the same diff.
7. **No commits on the user's behalf.** Run `git status` and
   `git diff` before staging. Never amend, force-push, or skip
   hooks.

## Module boundary rules

- Each `.cpp` file in `src/<area>/` implements exactly one
  `<class>` declared in the matching `<class>.h`.
- `AppController` is the only place that may instantiate the
  audio, model, and hotkey subsystems together. Other modules take
  their dependencies as constructor parameters.
- UI modules (Tray, Settings, Onboarding, Overlay) MUST NOT call
  `AudioCapture` or `WhisperEngine` directly. They communicate via
  `AppController` callbacks.

## Style

- Indentation: 4 spaces. Tabs are forbidden.
- Max line length: 100 columns for headers, 110 for `.cpp`.
- Trailing commas in initializer lists are required.
- `auto` is allowed when the type is obvious from the RHS; otherwise
  spell the type.
- Comments explain *why*, not *what*. Code that needs a *what*
  comment should be rewritten.

## How to run the full local validation

```powershell
# From the repo root, with the toolchain on PATH:
. C:\Users\jkmal\tools\env.ps1

cmake --preset debug
cmake --build build/debug -j4
ctest --test-dir build/debug --output-on-failure
python scripts/check_coverage.py build/debug
```

All four steps must succeed before a PR is ready for review.
