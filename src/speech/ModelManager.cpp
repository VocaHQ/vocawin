#include "speech/ModelManager.h"

#include <algorithm>
#include <fstream>
#include <system_error>

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
        {"large-v3",         "Large v3 (Multilingual, 1550M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
         2900 * kMB,  3900 * kMB},
        {"large-v3-turbo",   "Large v3 Turbo (Multilingual, 809M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
         809 * kMB,   1200 * kMB},
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

bool ModelManager::downloadModel(const std::string& modelId,
                                 const std::string& url,
                                 ProgressCallback onProgress) {
    const auto* meta = findById(modelId);
    if (meta == nullptr) {
        return false;
    }
    std::string effectiveUrl = url;
    if (effectiveUrl.empty()) {
        effectiveUrl = meta->url;
    }
    if (effectiveUrl.empty()) {
        return false;
    }

    constexpr const char* kFilePrefix = "file:///";
    constexpr std::size_t kFilePrefixLen = 8;
    if (effectiveUrl.compare(0, kFilePrefixLen, kFilePrefix) != 0) {
        // HTTPS / http downloads require WinHTTP, which is out of
        // scope for the MVP. Surface a clear failure to the caller.
        return false;
    }

    std::filesystem::path src = effectiveUrl.substr(kFilePrefixLen);

    std::error_code ec;
    std::filesystem::create_directories(modelsDir_, ec);
    if (ec) {
        return false;
    }
    const auto dst = getModelPath(modelId);

    std::ifstream in(src, std::ios::binary);
    if (!in.good()) {
        return false;
    }
    std::ofstream out(dst, std::ios::binary | std::ios::trunc);
    if (!out.good()) {
        return false;
    }
    constexpr std::size_t kBuf = 64 * 1024;
    char buf[kBuf];
    std::uint64_t total = 0;
    in.seekg(0, std::ios::end);
    const std::streamoff len = in.tellg();
    in.seekg(0, std::ios::beg);
    while (in) {
        in.read(buf, kBuf);
        const std::streamsize n = in.gcount();
        if (n <= 0) break;
        out.write(buf, n);
        total += static_cast<std::uint64_t>(n);
        if (onProgress && len > 0) {
            onProgress(static_cast<float>(
                static_cast<double>(total) /
                static_cast<double>(len)));
        }
    }
    if (onProgress) onProgress(1.0f);
    return out.good();
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
