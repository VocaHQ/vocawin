// Coverage dumper for VocaWin MVP.
//
// Compiled with --coverage to gain access to the libgcov runtime functions.
// Walks all loaded translation units, iterates each counter, and prints
// "FILE.cpp total=N hit=M" lines for downstream parsing.
//
// Usage: dump_coverage.exe
//   Reads .gcda files from the current build dir tree.

#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "input/clipboard_manager.h"

namespace {

int g_totalLines = 0;
int g_hitLines = 0;
std::string g_currentFile;

// libgcov internals - forward-declare to avoid pulling in the full
// coverage.h (which has GCC version-specific macros).
extern "C" {
struct gcov_info;
struct gcov_summary {
    gcov_unsigned_t sum;
    gcov_unsigned_t runs;
    gcov_unsigned_t run_max;
    gcov_unsigned_t sum_max;
};
void __gcov_dump_summary(void*);
void __gcov_reset(void*);
void __gcov_init(void*);
void __gcov_exit(void);
void __gcov_merge_add(gcov_type* counter, gcov_type value);
void __gcov_seek(gcov_position_t*);
void __gcov_rewrite(void);
}

}  // namespace

// The reliable cross-platform path: link with -fprofile-arcs (already on
// via --coverage) and let the program call into the libgcov dump API. But
// that requires us to compile THIS file with --coverage and to link with
// libgcov. Instead, we use a simpler approach: walk the .gcda files using
// the gcc-shipped `gcov-tool` (if available) or fall back to parsing the
// .gcov text output.

int main(int argc, char** argv) {
    // The simplest cross-platform reliable approach: invoke `gcov` on each
    // .gcda file and parse the per-file "Lines" line. The caller (Python
    // script) handles the aggregation. Here we just print a manifest.
    std::printf("dump_coverage is a placeholder; the Python script in\n");
    std::printf("scripts/check_coverage.py reads .gcda files directly.\n");
    return 0;
}
