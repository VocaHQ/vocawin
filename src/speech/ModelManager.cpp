#include "speech/ModelManager.h"

#include <algorithm>

namespace vocawin {

namespace {

constexpr std::size_t kMB = 1024ULL * 1024;

const std::vector<ModelManager::ModelInfo>& catalog() {
    static const std::vector<ModelManager::ModelInfo> kModels = {
        {"tiny.en",   "Tiny (English, 39M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
         75 * kMB,    273 * kMB},
        {"tiny",      "Tiny (Multilingual, 39M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
         75 * kMB,    273 * kMB},
        {"base.en",   "Base (English, 74M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
         142 * kMB,   388 * kMB},
        {"base",      "Base (Multilingual, 74M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
         142 * kMB,   388 * kMB},
        {"small.en",  "Small (English, 244M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
         466 * kMB,   852 * kMB},
        {"small",     "Small (Multilingual, 244M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
         466 * kMB,   852 * kMB},
        {"medium.en", "Medium (English, 769M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
         1500 * kMB,  2100 * kMB},
        {"medium",    "Medium (Multilingual, 769M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
         1500 * kMB,  2100 * kMB},
    };
    return kModels;
}

const ModelManager::ModelInfo* findById(const std::string& id) {
    const auto& m = catalog();
    auto it = std::find_if(m.begin(), m.end(),
                           [&id](const ModelManager::ModelInfo& mi) {
                               return mi.id == id;
                           });
    return it == m.end() ? nullptr : &(*it);
}

}  // namespace

ModelManager::ModelManager(std::filesystem::path modelsDir)
    : modelsDir_(std::move(modelsDir)) {}

std::vector<ModelManager::ModelInfo>
ModelManager::getAvailableModels() {
    return catalog();
}

std::vector<ModelManager::ModelInfo>
ModelManager::getLocalModels() const {
    std::vector<ModelInfo> out;
    for (const auto& m : catalog()) {
        const auto p = getModelPath(m.id);
        if (std::filesystem::exists(p)) {
            out.push_back(m);
        }
    }
    return out;
}

bool ModelManager::isModelDownloaded(const std::string& modelId) const {
    return std::filesystem::exists(getModelPath(modelId));
}

std::filesystem::path
ModelManager::getModelPath(const std::string& modelId) const {
    return modelsDir_ / ("ggml-" + modelId + ".bin");
}

ModelManager::ModelInfo
ModelManager::recommendModel(std::size_t ramBytes, std::size_t vramBytes,
                              bool hasGpu) {
    auto pick = [](const std::string& id) -> ModelInfo {
        const auto* m = findById(id);
        return m ? *m : ModelInfo{};
    };
    if (hasGpu) {
        if (vramBytes >= 8ULL * 1024 * kMB) return pick("medium.en");
        if (vramBytes >= 4ULL * 1024 * kMB) return pick("small.en");
        if (vramBytes >= 2ULL * 1024 * kMB) return pick("base.en");
        return pick("base.en");
    }
    if (ramBytes >= 32ULL * 1024 * kMB) return pick("small.en");
    if (ramBytes >= 16ULL * 1024 * kMB) return pick("base.en");
    if (ramBytes >= 8ULL * 1024 * kMB)  return pick("tiny.en");
    return pick("tiny.en");
}

}  // namespace vocawin
