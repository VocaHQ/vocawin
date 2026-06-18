// Compile-only smoke test: includes whisper.h and queries the C API default
// params. Does NOT load a model (would require a GGML file on disk and would
// dominate test runtime).
#include <cassert>
#include <cstdio>

#include "whisper.h"

int main() {
    const whisper_context_params cparams = whisper_context_default_params();
    assert(cparams.use_gpu == false || cparams.use_gpu == true);

    const whisper_full_params fparams = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    assert(fparams.strategy == WHISPER_SAMPLING_GREEDY);
    assert(fparams.n_threads > 0);

    std::printf("whisper.cpp linked OK; default n_threads=%d\n", fparams.n_threads);
    return 0;
}
