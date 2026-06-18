#include "audio/SilenceDetector.h"

#include <cmath>

namespace vocawin {

SilenceDetector::SilenceDetector()
    : threshold_(Config{}.threshold),
      silenceDurationMs_(Config{}.durationMs),
      lastLoudTime_(std::chrono::steady_clock::now()) {}

SilenceDetector::SilenceDetector(Config config)
    : threshold_(config.threshold),
      silenceDurationMs_(config.durationMs),
      lastLoudTime_(std::chrono::steady_clock::now()) {}

void SilenceDetector::feedSample(float sample) {
    const float amplitude = std::abs(sample);
    if (amplitude >= threshold_) {
        updateLoudTime();
        silent_ = false;
        timeoutFired_ = false;
    } else {
        silent_ = true;
        checkTimeout();
    }
}

void SilenceDetector::feedBuffer(const float* data, std::size_t len) {
    if (len == 0) {
        return;
    }
    // Compute RMS over the buffer.
    double sumSq = 0.0;
    for (std::size_t i = 0; i < len; ++i) {
        const double v = static_cast<double>(data[i]);
        sumSq += v * v;
    }
    const double rms = std::sqrt(sumSq / static_cast<double>(len));
    const float rmsF = static_cast<float>(rms);

    if (rmsF >= threshold_) {
        updateLoudTime();
        silent_ = false;
        timeoutFired_ = false;
    } else {
        silent_ = true;
        checkTimeout();
    }
}

void SilenceDetector::reset() {
    lastLoudTime_ = std::chrono::steady_clock::now();
    timeoutFired_ = false;
    silent_ = false;
}

std::chrono::milliseconds SilenceDetector::silenceDuration() const {
    const auto now = std::chrono::steady_clock::now();
    const auto dur = std::chrono::duration_cast<std::chrono::milliseconds>(now - lastLoudTime_);
    return dur < std::chrono::milliseconds(0) ? std::chrono::milliseconds(0) : dur;
}

void SilenceDetector::updateLoudTime() {
    lastLoudTime_ = std::chrono::steady_clock::now();
}

void SilenceDetector::checkTimeout() {
    if (timeoutFired_) {
        return;
    }
    if (silenceDuration().count() >= static_cast<std::int64_t>(silenceDurationMs_)) {
        timeoutFired_ = true;
        if (onSilenceTimeout) {
            onSilenceTimeout();
        }
    }
}

}  // namespace vocawin
