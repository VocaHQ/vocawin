#pragma once

#include <cstddef>
#include <string>

namespace vocawin {

// Detects available GPU compute backends (CUDA, Vulkan) and chooses the
// best one to drive whisper.cpp. Per SPEC \u00a74.2.3.
//
// On the MVP MinGW build whisper.cpp is CPU-only, so the detector
// always falls back to "CPU". When CUDA or Vulkan builds are enabled
// (VOCAWIN_CUDA=ON / VOCAWIN_VULKAN=ON) the detector picks the best
// available backend at runtime.
class GpuDetector {
public:
    struct Capabilities {
        bool cudaAvailable{false};
        bool vulkanAvailable{false};
        std::size_t vramBytes{0};
        std::string gpuName;
        std::string activeBackendName;
    };

    static Capabilities detect();
};

}  // namespace vocawin
