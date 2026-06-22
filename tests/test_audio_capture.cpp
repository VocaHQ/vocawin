#include <atomic>
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

    // 8. Win32: stop() called from the capture thread's onAudioLevel
    //    callback must not deadlock. Without the self-join guard, this
    //    raises std::system_error (resource_deadlock_would_occur) from
    //    thread_.join() on the calling thread, which calls
    //    std::terminate() and aborts the process. Skipped when no
    //    capture device is available.
#if defined(_WIN32)
    {
        vocawin::AudioCapture cap;
        std::atomic<bool> stopCalled{false};
        cap.onAudioLevel = [&cap, &stopCalled](float) {
            if (!stopCalled.exchange(true)) {
                cap.stop();
            }
        };
        vocawin::AudioCapture::Config cfg;
        if (cap.start(cfg)) {
            for (int i = 0; i < 50 && cap.isCapturing(); ++i) {
                std::this_thread::sleep_for(std::chrono::milliseconds(100));
            }
            assert(!cap.isCapturing());
        }
    }
#endif

    return 0;
}
