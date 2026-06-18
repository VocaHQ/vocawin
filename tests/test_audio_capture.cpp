#include <cassert>
#include <chrono>
#include <cstdint>
#include <thread>
#include <vector>

#include "audio/AudioCapture.h"

int main() {
    // 1. Default config matches spec.
    {
        vocawin::AudioCapture::Config cfg;
        assert(cfg.sampleRate == 16000);
        assert(cfg.channels == 1);
        assert(cfg.bufferDurationMs == 100);
        assert(cfg.deviceIndex == -1);
    }

    // 2. Non-Win32 stub: start returns false, isCapturing false.
#if !defined(_WIN32)
    {
        vocawin::AudioCapture cap;
        vocawin::AudioCapture::Config cfg;
        assert(!cap.start(cfg));
        assert(!cap.isCapturing());
        cap.stop();  // no crash
    }
#endif

    // 3. Win32: start may succeed if a capture device is present, fail
    //    otherwise. Either way the post-state must be consistent.
#if defined(_WIN32)
    {
        vocawin::AudioCapture cap;
        vocawin::AudioCapture::Config cfg;
        const bool started = cap.start(cfg);
        assert(cap.isCapturing() == started);
        cap.stop();
        assert(!cap.isCapturing());
    }
#endif

    // 4. Buffer is empty before start and after clearBuffer.
    {
        vocawin::AudioCapture cap;
        assert(cap.getBuffer().empty());
        cap.clearBuffer();
        assert(cap.getBuffer().empty());
    }

    // 5. Win32: callbacks are settable.
    {
        vocawin::AudioCapture cap;
        std::atomic<int> dataCount{0};
        cap.onAudioData = [&dataCount](const float*, std::size_t) {
            dataCount.fetch_add(1);
        };
        cap.onAudioLevel = [](float) {};
        cap.onSilenceDetected = []() {};
        (void)dataCount;
    }

    // 6. Win32: enumerateDevices returns the device names (may be empty).
    {
        const auto devs = vocawin::AudioCapture::enumerateDevices();
        // No assertion on content - depends on hardware.
        (void)devs;
    }

    // 7. Win32: start, then sleep briefly, then check buffer has data.
    //    (May not capture anything if no mic, but should not crash.)
#if defined(_WIN32)
    {
        vocawin::AudioCapture cap;
        vocawin::AudioCapture::Config cfg;
        if (cap.start(cfg)) {
            std::this_thread::sleep_for(std::chrono::milliseconds(200));
            const auto buf = cap.getBuffer();
            // Buffer may or may not have samples depending on mic; just
            // assert no crash and the call is well-defined.
            (void)buf;
            cap.stop();
        }
    }
#endif

    return 0;
}
