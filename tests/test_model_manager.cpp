#include <cassert>
#include <cstddef>
#include <filesystem>
#include <fstream>
#include <string>

#include "speech/ModelManager.h"

int main() {
    // 1. Catalog is non-empty.
    {
        const auto models = vocawin::ModelManager::getAvailableModels();
        assert(!models.empty());
    }

    // 2. Catalog includes the tiny model.
    {
        const auto models = vocawin::ModelManager::getAvailableModels();
        bool foundTiny = false;
        for (const auto& m : models) {
            if (m.id == "tiny.en") {
                foundTiny = true;
                assert(!m.url.empty());
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

    return 0;
}
