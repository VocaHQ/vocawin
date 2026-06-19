#pragma once

#include <chrono>
#include <cstdint>
#include <functional>

namespace vocawin {

// RMS-based voice activity detection.
//
// Tracks incoming audio levels; once silence has persisted for at least
// `durationMs`, fires `onSilenceTimeout` exactly once until either
// `feedBuffer`/`feedSample` reports a loud frame or `reset()` is called.
//
// Implementation per SPEC \u00a74.2.8 (page ~560).
class SilenceDetector {
public:
    struct Config {
        float threshold = 0.01f;      // RMS threshold; frames with RMS < threshold are "silent"
        std::uint32_t durationMs = 2000;  // Silence duration before firing onSilenceTimeout
    };

    explicit SilenceDetector(Config config);
    SilenceDetector();  // uses Config defaults

    // Feed a single sample (per-sample amplitude check against threshold).
    void feedSample(float sample);

    // Feed a buffer; RMS is computed over the buffer, then compared to threshold.
    void feedBuffer(const float* data, std::size_t len);

    // Re-arm: clear timeoutFired and reset lastLoudTime to now.
    void reset();

    void applyConfig(const Config& c) {
        threshold_ = c.threshold;
        silenceDurationMs_ = c.durationMs;
    }
    float threshold() const { return threshold_; }
    std::uint32_t durationMs() const { return silenceDurationMs_; }

    bool isSilent() const { return silent_; }
    std::chrono::milliseconds silenceDuration() const;

    // Fires exactly once per silence period (until reset or loud frame).
    std::function<void()> onSilenceTimeout;

private:
    void updateLoudTime();
    void checkTimeout();

    float threshold_;
    std::uint32_t silenceDurationMs_;
    std::chrono::steady_clock::time_point lastLoudTime_;
    bool silent_{false};
    bool timeoutFired_{false};
};

}  // namespace vocawin
