#include <cassert>
#include <chrono>
#include <thread>
#include <vector>

#include "audio/SilenceDetector.h"

int main() {
    // 1. Loud buffer -> not silent.
    {
        vocawin::SilenceDetector sd({0.01f, 50});
        std::vector<float> loud(160, 0.5f);  // 10ms @ 16kHz, RMS ~ 0.5
        sd.feedBuffer(loud.data(), loud.size());
        assert(!sd.isSilent());
    }

    // 2. Silent buffer -> silent.
    {
        vocawin::SilenceDetector sd({0.01f, 50});
        std::vector<float> silent(160, 0.0f);
        sd.feedBuffer(silent.data(), silent.size());
        assert(sd.isSilent());
        assert(sd.silenceDuration() >= std::chrono::milliseconds(0));
    }

    // 3. Timeout fires after silence > durationMs.
    {
        vocawin::SilenceDetector sd({0.01f, 50});
        int callCount = 0;
        sd.onSilenceTimeout = [&callCount]() { ++callCount; };

        // Loud: resets lastLoudTime
        std::vector<float> loud(160, 0.5f);
        sd.feedBuffer(loud.data(), loud.size());
        assert(callCount == 0);

        // Silent enough to fire timeout (>50ms)
        std::vector<float> silent(80, 0.0f);  // 5ms
        for (int i = 0; i < 20 && callCount == 0; ++i) {
            sd.feedBuffer(silent.data(), silent.size());
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        assert(callCount == 1);
    }

    // 4. Timeout fires once per silence period.
    {
        vocawin::SilenceDetector sd({0.01f, 50});
        int callCount = 0;
        sd.onSilenceTimeout = [&callCount]() { ++callCount; };

        std::vector<float> loud(160, 0.5f);
        sd.feedBuffer(loud.data(), loud.size());

        std::vector<float> silent(80, 0.0f);
        for (int i = 0; i < 30 && callCount == 0; ++i) {
            sd.feedBuffer(silent.data(), silent.size());
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        assert(callCount == 1);
        // Keep feeding silence - should not fire again
        for (int i = 0; i < 5; ++i) {
            sd.feedBuffer(silent.data(), silent.size());
            std::this_thread::sleep_for(std::chrono::milliseconds(15));
        }
        assert(callCount == 1);
    }

    // 5. Reset re-arms the timeout.
    {
        vocawin::SilenceDetector sd({0.01f, 50});
        int callCount = 0;
        sd.onSilenceTimeout = [&callCount]() { ++callCount; };

        std::vector<float> loud(160, 0.5f);
        sd.feedBuffer(loud.data(), loud.size());
        std::vector<float> silent(80, 0.0f);
        for (int i = 0; i < 30 && callCount == 0; ++i) {
            sd.feedBuffer(silent.data(), silent.size());
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        assert(callCount == 1);

        sd.reset();
        // Feed loud to push lastLoudTime forward, then silence
        sd.feedBuffer(loud.data(), loud.size());
        for (int i = 0; i < 30 && callCount == 1; ++i) {
            sd.feedBuffer(silent.data(), silent.size());
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        assert(callCount == 2);
    }

    // 6. Loud after silence resets the silence clock.
    {
        vocawin::SilenceDetector sd({0.01f, 2000});
        std::vector<float> silent(80, 0.0f);
        sd.feedBuffer(silent.data(), silent.size());
        assert(sd.isSilent());
        // Loud resets the clock
        std::vector<float> loud(160, 0.5f);
        sd.feedBuffer(loud.data(), loud.size());
        assert(!sd.isSilent());
    }

    // 7. Threshold boundary: amplitude at exactly the threshold is loud.
    {
        vocawin::SilenceDetector sd({0.5f, 50});
        // DC signal of 0.5 -> RMS = 0.5 == threshold
        std::vector<float> boundary(160, 0.5f);
        sd.feedBuffer(boundary.data(), boundary.size());
        assert(!sd.isSilent());
    }

    // 8. feedSample respects per-sample threshold.
    {
        vocawin::SilenceDetector sd({0.5f, 50});
        sd.feedSample(0.7f);
        assert(!sd.isSilent());
        sd.feedSample(0.1f);
        assert(sd.isSilent());
    }

    return 0;
}
