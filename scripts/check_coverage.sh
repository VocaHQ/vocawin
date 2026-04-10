#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/build/coverage"

if command -v llvm-profdata >/dev/null 2>&1; then
  LLVM_PROFDATA="$(command -v llvm-profdata)"
elif [[ -x "/opt/homebrew/opt/llvm/bin/llvm-profdata" ]]; then
  LLVM_PROFDATA="/opt/homebrew/opt/llvm/bin/llvm-profdata"
else
  echo "llvm-profdata not found. Install LLVM tooling first."
  exit 1
fi

if command -v llvm-cov >/dev/null 2>&1; then
  LLVM_COV="$(command -v llvm-cov)"
elif [[ -x "/opt/homebrew/opt/llvm/bin/llvm-cov" ]]; then
  LLVM_COV="/opt/homebrew/opt/llvm/bin/llvm-cov"
else
  echo "llvm-cov not found. Install LLVM tooling first."
  exit 1
fi

cmake --preset coverage
cmake --build --preset coverage

rm -f "$BUILD_DIR"/*.profraw

LLVM_PROFILE_FILE="$BUILD_DIR/test-single-instance.profraw" "$BUILD_DIR/tests/test_single_instance"
LLVM_PROFILE_FILE="$BUILD_DIR/test-settings-store.profraw" "$BUILD_DIR/tests/test_settings_store"
LLVM_PROFILE_FILE="$BUILD_DIR/test-app-controller.profraw" "$BUILD_DIR/tests/test_app_controller"
LLVM_PROFILE_FILE="$BUILD_DIR/test-logger.profraw" "$BUILD_DIR/tests/test_logger"
LLVM_PROFILE_FILE="$BUILD_DIR/test-tray-icon.profraw" "$BUILD_DIR/tests/test_tray_icon"

PROFDATA="$BUILD_DIR/coverage.profdata"

"$LLVM_PROFDATA" merge -sparse "$BUILD_DIR"/*.profraw -o "$PROFDATA"

BINARIES=(
  "$BUILD_DIR/tests/test_single_instance"
  "$BUILD_DIR/tests/test_settings_store"
  "$BUILD_DIR/tests/test_app_controller"
  "$BUILD_DIR/tests/test_logger"
  "$BUILD_DIR/tests/test_tray_icon"
)

OBJECT_ARGS=()
for bin in "${BINARIES[@]}"; do
  OBJECT_ARGS+=("-object" "$bin")
done

"$LLVM_COV" report "${BINARIES[0]}" \
  "${OBJECT_ARGS[@]}" \
  -instr-profile="$PROFDATA" \
  "$ROOT_DIR/src/app/AppController.cpp" \
  "$ROOT_DIR/src/app/SingleInstance.cpp" \
  "$ROOT_DIR/src/config/SettingsStore.cpp" \
  "$ROOT_DIR/src/util/Logger.cpp" \
  "$ROOT_DIR/src/ui/TrayIcon.cpp"

TOTAL_LINE="$("$LLVM_COV" report "${BINARIES[0]}" "${OBJECT_ARGS[@]}" -instr-profile="$PROFDATA" "$ROOT_DIR/src/app/AppController.cpp" "$ROOT_DIR/src/app/SingleInstance.cpp" "$ROOT_DIR/src/config/SettingsStore.cpp" "$ROOT_DIR/src/util/Logger.cpp" "$ROOT_DIR/src/ui/TrayIcon.cpp" | awk '/TOTAL/ {print}')"

TOTAL_VALUE="$(python3 - <<PY
import re
line = """$TOTAL_LINE"""
matches = re.findall(r"(\d+\.\d+)%", line)
if len(matches) < 3:
    raise SystemExit("Could not parse TOTAL line coverage")
print(matches[2])
PY
)"

python3 - <<PY
value = float("$TOTAL_VALUE")
if value < 80.0:
    raise SystemExit(f"Coverage check failed: {value:.2f}% < 80.00%")
print(f"Coverage check passed: {value:.2f}% >= 80.00%")
PY
