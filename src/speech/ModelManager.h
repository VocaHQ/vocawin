#pragma once

#include <cstddef>
#include <filesystem>
#include <string>
#include <vector>

namespace vocawin {

// Whisper model catalog, path management, and hardware-based
// recommendation. Per SPEC \u00a74.2.4 (page ~375).
class ModelManager {
public:
    struct ModelInfo {
        std::string id;             // "tiny.en", "base.en", "small.en", "medium.en"
        std::string displayName;    // "Tiny (39M params)"
        std::string url;            // HuggingFace download URL
        std::size_t fileSizeBytes;  // Size of the .bin file
        std::size_t ramRequiredBytes;  // Approx. RAM needed to load + run
    };

    explicit ModelManager(std::filesystem::path modelsDir);

    // Static catalog of all available models (downloaded or not).
    static std::vector<ModelInfo> getAvailableModels();

    // Returns the subset of getAvailableModels() whose files are present
    // on disk under modelsDir.
    std::vector<ModelInfo> getLocalModels() const;

    // True if the .bin file for the given model id is present on disk.
    bool isModelDownloaded(const std::string& modelId) const;

    // Resolves the canonical on-disk path for a model id. The file may or
    // may not exist (callers should check isModelDownloaded first).
    std::filesystem::path getModelPath(const std::string& modelId) const;

    // Recommend the best model id for the given hardware profile. Mirrors
    // the VocaMac algorithm: GPU path prefers CUDA/Vulkan VRAM buckets;
    // CPU path falls back to RAM buckets. Returns one of the catalog ids.
    static ModelInfo recommendModel(std::size_t ramBytes,
                                    std::size_t vramBytes,
                                    bool hasGpu);

    const std::filesystem::path& modelsDir() const { return modelsDir_; }

private:
    std::filesystem::path modelsDir_;
};

}  // namespace vocawin
