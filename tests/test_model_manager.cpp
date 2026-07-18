#include <cassert>
#include <cstddef>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

#include "speech/ModelManager.h"

int main() {
    // 1. Catalog is non-empty.
    {
        const auto models = vocawin::ModelManager::getAvailableModels();
        assert(!models.empty());
    }

    // 2. Catalog includes the tiny model with an HTTPS catalog URL.
    {
        const auto models = vocawin::ModelManager::getAvailableModels();
        bool foundTiny = false;
        for (const auto& m : models) {
            if (m.id == "tiny.en") {
                foundTiny = true;
                assert(!m.url.empty());
                assert(m.url.compare(0, 8, "https://") == 0);
                assert(m.fileSizeBytes > 0);
                assert(m.ramRequiredBytes > 0);
                assert(!m.displayName.empty());
            }
        }
        assert(foundTiny);
    }

    // 3. getModelPath returns <dir>/ggml-<id>.bin.
    {
        const std::filesystem::path dir = "build/test-model-manager";
        std::filesystem::remove_all(dir);
        vocawin::ModelManager mm(dir);
        const auto p = mm.getModelPath("base.en");
        assert(p.filename() == "ggml-base.en.bin");
        assert(p.parent_path() == dir);
    }

    // 4. isModelDownloaded returns false when file is missing.
    {
        const std::filesystem::path dir = "build/test-model-manager";
        std::filesystem::remove_all(dir);
        vocawin::ModelManager mm(dir);
        assert(!mm.isModelDownloaded("tiny.en"));
    }

    // 5. isModelDownloaded returns true when the file exists.
    {
        const std::filesystem::path dir = "build/test-model-manager";
        std::filesystem::create_directories(dir);
        const auto p = dir / "ggml-tiny.en.bin";
        std::ofstream out(p);
        out << "fake model bytes";
        out.close();
        vocawin::ModelManager mm(dir);
        assert(mm.isModelDownloaded("tiny.en"));
        std::filesystem::remove_all(dir);
    }

    // 6. getLocalModels enumerates only present files.
    {
        const std::filesystem::path dir = "build/test-model-manager";
        std::filesystem::create_directories(dir);
        std::ofstream(dir / "ggml-tiny.en.bin") << "a";
        std::ofstream(dir / "ggml-base.en.bin") << "b";
        vocawin::ModelManager mm(dir);
        const auto locals = mm.getLocalModels();
        assert(locals.size() == 2);
        std::filesystem::remove_all(dir);
    }

    // 7. Recommendation algorithm: CPU with low RAM -> tiny.
    {
        const auto rec = vocawin::ModelManager::recommendModel(
            /*ramBytes=*/4ULL * 1024 * 1024 * 1024,
            /*vramBytes=*/0,
            /*hasGpu=*/false);
        assert(rec.id == "tiny.en");
    }

    // 8. Recommendation: CPU with 16GB RAM -> base.
    {
        const auto rec = vocawin::ModelManager::recommendModel(
            16ULL * 1024 * 1024 * 1024, 0, false);
        assert(rec.id == "base.en");
    }

    // 9. Recommendation: GPU with 8GB VRAM -> medium.
    {
        const auto rec = vocawin::ModelManager::recommendModel(
            0, 8ULL * 1024 * 1024 * 1024, true);
        assert(rec.id == "medium.en");
    }

    // 10. Recommendation: GPU with 2GB VRAM -> base.
    {
        const auto rec = vocawin::ModelManager::recommendModel(
            0, 2ULL * 1024 * 1024 * 1024, true);
        assert(rec.id == "base.en");
    }

    // 11. Recommendation: GPU with 4GB VRAM -> small.
    {
        const auto rec = vocawin::ModelManager::recommendModel(
            0, 4ULL * 1024 * 1024 * 1024, true);
        assert(rec.id == "small.en");
    }

    // 12. downloadModel via file://: copy local fixture with progress.
    {
        const std::filesystem::path dir = "build/test-model-manager";
        std::filesystem::remove_all(dir);
        vocawin::ModelManager mm(dir);

        const std::filesystem::path srcDir = "build/test-model-manager-src";
        std::filesystem::create_directories(srcDir);
        const std::filesystem::path src = srcDir / "ggml-tiny.en.bin";
        // >= 1 KiB so content is non-trivial for size checks downstream.
        std::string payload(2048, 'A');
        std::ofstream(src, std::ios::binary).write(payload.data(),
            static_cast<std::streamsize>(payload.size()));

        const std::string fileUrl = "file:///" +
            std::filesystem::absolute(src).generic_string();
        bool progressed = false;
        float lastP = -1.0f;
        const bool ok = mm.downloadModel("tiny.en", fileUrl,
            [&](float p) {
                if (p > 0.0f && p <= 1.0f) progressed = true;
                lastP = p;
            });
        assert(ok);
        assert(progressed);
        assert(lastP >= 0.99f);
        assert(mm.isModelDownloaded("tiny.en"));
        // Atomic write leaves no .part residue on success.
        assert(!std::filesystem::exists(mm.getModelPath("tiny.en").string() + ".part"));
        const auto sz = std::filesystem::file_size(mm.getModelPath("tiny.en"));
        assert(sz == payload.size());

        std::filesystem::remove_all(srcDir);
        std::filesystem::remove_all(dir);
    }

    // 13. downloadModel: unknown model id returns false.
    {
        const std::filesystem::path dir = "build/test-model-manager";
        std::filesystem::create_directories(dir);
        vocawin::ModelManager mm(dir);
        assert(!mm.downloadModel("nope", "file:///nope", nullptr));
        std::filesystem::remove_all(dir);
    }

    // 14. HTTPS download of an unreachable host fails cleanly (not success).
    {
        const std::filesystem::path dir = "build/test-model-manager-https-fail";
        std::filesystem::remove_all(dir);
        vocawin::ModelManager mm(dir);
        bool progressOnFail = false;
        const bool ok = mm.downloadModel(
            "tiny.en",
            "https://127.0.0.1:1/this-port-is-closed/ggml-tiny.en.bin",
            [&](float) { progressOnFail = true; });
        assert(!ok);
        assert(!mm.isModelDownloaded("tiny.en"));
        // May or may not fire progress before connection fails; either is fine
        // as long as we did not claim success or leave a final model file.
        (void)progressOnFail;
        assert(!std::filesystem::exists(mm.getModelPath("tiny.en")));
        std::filesystem::remove_all(dir);
    }

    // 15. Unsupported scheme fails (never silent success).
    {
        const std::filesystem::path dir = "build/test-model-manager-scheme";
        std::filesystem::remove_all(dir);
        vocawin::ModelManager mm(dir);
        assert(!mm.downloadModel("tiny.en", "ftp://example.com/x.bin", nullptr));
        assert(!mm.isModelDownloaded("tiny.en"));
        std::filesystem::remove_all(dir);
    }

    // 16. Empty catalog URL override with missing file:// source fails.
    {
        const std::filesystem::path dir = "build/test-model-manager-missing-src";
        std::filesystem::remove_all(dir);
        vocawin::ModelManager mm(dir);
        assert(!mm.downloadModel(
            "tiny.en",
            "file:///Z:/definitely-does-not-exist-vocawin-fixture.bin",
            nullptr));
        assert(!mm.isModelDownloaded("tiny.en"));
        std::filesystem::remove_all(dir);
    }

    return 0;
}
