#pragma once

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <mutex>
#include <thread>
#include <vector>

namespace vocawin {

// WASAPI microphone capture at 16kHz mono Float32 (the format whisper.cpp
// expects). Per SPEC \u00a74.2.2. Uses raw WASAPI (not SDL/miniaudio) to keep
// the dependency surface minimal. On non-Win32 platforms `start()` is a
// no-op that returns false.
class AudioCapture {
public:
    struct Config {
        std::uint32_t sampleRate = 16000;   // Whisper requirement
        std::uint32_t channels = 1;         // Mono
        std::uint32_t bufferDurationMs = 100;
        int deviceIndex = -1;               // -1 = system default
    };

    AudioCapture();
    ~AudioCapture();

    AudioCapture(const AudioCapture&) = delete;
    AudioCapture& operator=(const AudioCapture&) = delete;

    // Begin capture on the configured device. Returns true on success.
    bool start(Config config);
    void stop();

    bool isCapturing() const { return capturing_.load(); }

    // Return a copy of all collected samples since capture began (or the
    // last getBuffer/clearBuffer).
    std::vector<float> getBuffer();

    // Discard the collected buffer.
    void clearBuffer();

    // Callbacks fired on the WASAPI capture thread:
    std::function<void(const float*, std::size_t)> onAudioData;
    std::function<void(float)> onAudioLevel;     // RMS 0..1
    std::function<void()> onSilenceDetected;

    // Enumerate available capture device display names. Empty on non-Win32.
    static std::vector<std::wstring> enumerateDevices();

private:
    void captureThread();
    void accumulate(const float* data, std::size_t count);

    Config config_{};
    std::atomic<bool> capturing_{false};
    std::atomic<bool> stopRequested_{false};
    std::thread thread_;
    std::vector<float> buffer_;
    std::mutex bufferMutex_;

#if defined(_WIN32)
    void* audioClient_{nullptr};      // IAudioClient*
    void* captureClient_{nullptr};    // IAudioCaptureClient*
    void* device_{nullptr};           // IMMDevice*
#endif
};

}  // namespace vocawin
