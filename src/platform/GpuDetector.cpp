#include "platform/GpuDetector.h"

#include <fstream>
#include <sstream>

namespace vocawin {

GpuDetector::Capabilities GpuDetector::detect() {
    Capabilities caps;
    caps.gpuName = "Generic GPU";
    caps.activeBackendName = "CPU";
    caps.vramBytes = 0;

#if defined(_WIN32)
    // Heuristic: look for an nvidia-smi installation to flag CUDA.
    // (The actual inference backend is determined at link time by the
    // GGML_CUDA / GGML_VULKAN flags passed to whisper.cpp.)
    std::ifstream smi("C:/Program Files/NVIDIA Corporation/NVSMI/nvidia-smi.exe");
    if (smi.good()) {
        caps.cudaAvailable = true;
        caps.gpuName = "NVIDIA GPU (nvidia-smi present)";
        caps.activeBackendName = "CUDA";
    }
#endif
    return caps;
}

}  // namespace vocawin
