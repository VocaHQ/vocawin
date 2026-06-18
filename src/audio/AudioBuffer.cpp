#include "audio/AudioBuffer.h"

#include <cstring>

namespace vocawin {

AudioBuffer::AudioBuffer(std::size_t capacitySamples)
    : buffer_(capacitySamples, 0.0f), capacity_(capacitySamples) {}

bool AudioBuffer::push(const float* data, std::size_t count) {
    if (count == 0) {
        return true;
    }
    if (count > capacity_) {
        return false;
    }
    const std::size_t write = writePos_.load(std::memory_order_relaxed);
    const std::size_t read = readPos_.load(std::memory_order_acquire);
    const std::size_t used = write - read;
    if (used + count > capacity_) {
        return false;
    }
    const std::size_t start = write % capacity_;
    if (start + count <= capacity_) {
        std::memcpy(buffer_.data() + start, data, count * sizeof(float));
    } else {
        const std::size_t firstPart = capacity_ - start;
        std::memcpy(buffer_.data() + start, data, firstPart * sizeof(float));
        std::memcpy(buffer_.data(), data + firstPart, (count - firstPart) * sizeof(float));
    }
    writePos_.store(write + count, std::memory_order_release);
    return true;
}

std::size_t AudioBuffer::pop(float* out, std::size_t maxCount) {
    const std::size_t read = readPos_.load(std::memory_order_relaxed);
    const std::size_t write = writePos_.load(std::memory_order_acquire);
    const std::size_t used = write - read;
    const std::size_t toCopy = (maxCount < used) ? maxCount : used;
    if (toCopy == 0) {
        return 0;
    }
    const std::size_t start = read % capacity_;
    if (start + toCopy <= capacity_) {
        std::memcpy(out, buffer_.data() + start, toCopy * sizeof(float));
    } else {
        const std::size_t firstPart = capacity_ - start;
        std::memcpy(out, buffer_.data() + start, firstPart * sizeof(float));
        std::memcpy(out + firstPart, buffer_.data(), (toCopy - firstPart) * sizeof(float));
    }
    readPos_.store(read + toCopy, std::memory_order_release);
    return toCopy;
}

std::size_t AudioBuffer::available() const {
    const std::size_t write = writePos_.load(std::memory_order_acquire);
    const std::size_t read = readPos_.load(std::memory_order_acquire);
    return write - read;
}

void AudioBuffer::clear() {
    writePos_.store(0, std::memory_order_release);
    readPos_.store(0, std::memory_order_release);
}

}  // namespace vocawin
