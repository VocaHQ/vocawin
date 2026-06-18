#pragma once

#include <atomic>
#include <cstddef>
#include <vector>

namespace vocawin {

// Single-producer single-consumer lock-free ring buffer for float audio samples.
//
// Designed to bridge the WASAPI capture thread (producer) and the inference
// thread (consumer) per SPEC \u00a73.2: "Lock-free ring buffer between audio
// capture and inference threads."
//
// The buffer is non-streaming: push() rejects when there isn't enough room
// to hold the full payload (no partial overwrites). This keeps semantics
// simple and matches the MVP flow where the entire utterance is captured
// before transcription starts.
class AudioBuffer {
public:
    explicit AudioBuffer(std::size_t capacitySamples);

    // Producer: append samples. Returns false if there is not enough free
    // space for the full request (no partial push).
    bool push(const float* data, std::size_t count);

    // Consumer: copy up to maxCount samples into out. Returns the actual
    // number of samples copied.
    std::size_t pop(float* out, std::size_t maxCount);

    // Number of samples available to pop.
    std::size_t available() const;

    // Total capacity in samples.
    std::size_t capacity() const { return capacity_; }

    // Reset read/write positions to 0. Discards any pending samples.
    void clear();

private:
    std::vector<float> buffer_;
    std::atomic<std::size_t> writePos_{0};
    std::atomic<std::size_t> readPos_{0};
    std::size_t capacity_;
};

}  // namespace vocawin
