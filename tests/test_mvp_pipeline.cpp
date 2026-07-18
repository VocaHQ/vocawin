// MVP pipeline: download (file:// or optional HTTPS) → load → transcribe.
// Default ctest path uses a local fixture via file:// so it stays offline.
// Set VOCAWIN_E2E=1 to also exercise HTTPS download of tiny.en (~75MB).

#include <cassert>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>

#include "speech/ModelManager.h"
#include "speech/WhisperEngine.h"

namespace {

std::vector<float> makeTonePcm(float seconds, float hz, int sampleRate) {
    const std::size_t n = static_cast<std::size_t>(seconds * sampleRate);
    std::vector<float> pcm(n);
    constexpr float kTwoPi = 6.28318530718f;
    for (std::size_t i = 0; i < n; ++i) {
        pcm[i] = 0.2f * std::sin(kTwoPi * hz * static_cast<float>(i) /
                                 static_cast<float>(sampleRate));
    }
    return pcm;
}

bool envEnabled(const char* name) {
    const char* v = std::getenv(name);
    return v != nullptr && v[0] == '1' && v[1] == '\0';
}

}  // namespace

int main() {
    const std::filesystem::path modelsDir = "build/test-mvp-pipeline/models";
    std::error_code ec;
    std::filesystem::create_directories(modelsDir, ec);

    vocawin::ModelManager mm(modelsDir);

    // --- Always: file:// progress + durable write ---
    {
        const std::filesystem::path srcDir = "build/test-mvp-pipeline/src";
        std::filesystem::create_directories(srcDir, ec);
        const auto src = srcDir / "fixture.bin";
        std::string bytes(8192, 'M');
        std::ofstream(src, std::ios::binary)
            .write(bytes.data(), static_cast<std::streamsize>(bytes.size()));

        // Install as base.en via file:// so we do not clobber a real tiny.en.
        const std::string fileUrl =
            "file:///" + std::filesystem::absolute(src).generic_string();
        bool progressed = false;
        const bool ok = mm.downloadModel("base.en", fileUrl, [&](float p) {
            if (p > 0.0f && p <= 1.0f) {
                progressed = true;
            }
        });
        assert(ok);
        assert(progressed);
        assert(mm.isModelDownloaded("base.en"));
        assert(std::filesystem::file_size(mm.getModelPath("base.en")) ==
               bytes.size());
        std::cout << "file:// download ok\n";
    }

    // --- Always: HTTPS clean failure is not silent success ---
    {
        const bool ok = mm.downloadModel(
            "small.en",
            "https://127.0.0.1:1/closed-port/ggml-small.en.bin",
            nullptr);
        assert(!ok);
        assert(!mm.isModelDownloaded("small.en"));
        std::cout << "https clean-fail ok\n";
    }

    // --- Optional: real HTTPS tiny.en + load + transcribe ---
    // Real HTTPS download + load + transcribe only when VOCAWIN_E2E=1 so
    // default ctest stays fast and offline. A prior download under modelsDir
    // is reused when present.
    if (envEnabled("VOCAWIN_E2E")) {
        const bool alreadyHaveTiny = mm.isModelDownloaded("tiny.en") &&
            std::filesystem::file_size(mm.getModelPath("tiny.en")) > 1'000'000;
        if (!alreadyHaveTiny) {
            std::cout << "HTTPS downloading tiny.en (~75MB)...\n";
            float last = -1.0f;
            const bool ok = mm.downloadModel("tiny.en", "", [&](float p) {
                if (p - last >= 0.1f || p >= 0.99f) {
                    last = p;
                    std::cout << "  progress " << static_cast<int>(p * 100)
                              << "%\n";
                }
            });
            if (!ok) {
                std::cerr << "HTTPS tiny.en download failed "
                             "(network blocked?)\n";
                // Plan allows clean HTTPS failure when network is blocked.
                return 0;
            }
            assert(mm.isModelDownloaded("tiny.en"));
            assert(std::filesystem::file_size(mm.getModelPath("tiny.en")) >
                   1'000'000);
            std::cout << "HTTPS tiny.en download ok, bytes="
                      << std::filesystem::file_size(mm.getModelPath("tiny.en"))
                      << "\n";
        } else {
            std::cout << "reusing cached tiny.en\n";
        }

        vocawin::WhisperEngine eng;
        const bool loaded = eng.loadModel(
            mm.getModelPath("tiny.en"),
            vocawin::WhisperEngine::GpuBackend{"CPU", "Generic CPU"},
            2);
        assert(loaded);
        assert(eng.isModelLoaded());
        eng.setLanguage("en");

        // 1s of tone: must not crash; text may be empty (noise ≠ speech).
        const auto pcm = makeTonePcm(1.0f, 440.0f, 16000);
        const auto result = eng.transcribe(pcm);
        // Loaded model + non-empty PCM must return a Result, not nullopt.
        assert(result.has_value());
        std::cout << "transcribe ok, text_len=" << result->text.size()
                  << " confidence=" << result->confidence << "\n";

        eng.unloadModel();
        assert(!eng.isModelLoaded());
        std::cout << "load+transcribe pipeline ok\n";
    } else {
        std::cout << "skip real-model load (set VOCAWIN_E2E=1 to download)\n";
    }

    return 0;
}
