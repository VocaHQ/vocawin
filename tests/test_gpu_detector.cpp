#include <cassert>
#include <string>

#include "platform/GpuDetector.h"

int main() {
    using namespace vocawin;

    GpuDetector::Capabilities caps = GpuDetector::detect();

    // On MinGW we build without CUDA/Vulkan; CPU path is the fallback.
    // The detector must always report a backend, never empty.
    assert(!caps.activeBackendName.empty());
    assert(!caps.gpuName.empty());
    if (!caps.cudaAvailable && !caps.vulkanAvailable) {
        assert(caps.activeBackendName == "CPU");
    } else {
        assert(caps.activeBackendName == "CUDA" ||
               caps.activeBackendName == "Vulkan" ||
               caps.activeBackendName == "CPU");
    }
    return 0;
}
