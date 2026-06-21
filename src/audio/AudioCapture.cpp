#include "audio/AudioCapture.h"

#include <cmath>
#include <mutex>

#if defined(_WIN32)
// MinGW fix: initguid.h MUST be included before mmdeviceapi.h so the
// IID_/CLSID_ symbols are emitted as static data (otherwise __uuidof is
// required, which is not reliable in MinGW).
#include <initguid.h>
#include <windows.h>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <propkey.h>
#include <functiondiscoverykeys.h>
#endif

namespace vocawin {

AudioCapture::AudioCapture() = default;

AudioCapture::~AudioCapture() {
    stop();
}

#if defined(_WIN32)
// Minimal COM smart pointer (MinGW lacks WRL::ComPtr).
template <typename T>
class ComPtr {
public:
    ComPtr() = default;
    ~ComPtr() { if (p_) p_->Release(); }
    ComPtr(const ComPtr&) = delete;
    ComPtr& operator=(const ComPtr&) = delete;
    ComPtr(ComPtr&& o) noexcept : p_(o.p_) { o.p_ = nullptr; }
    ComPtr& operator=(ComPtr&& o) noexcept { reset(o.p_); o.p_ = nullptr; return *this; }
    void reset(T* p = nullptr) { if (p_) p_->Release(); p_ = p; }
    T* release() noexcept { T* p = p_; p_ = nullptr; return p; }
    T* get() const { return p_; }
    T** put() { reset(); return &p_; }
    T* operator->() const { return p_; }
    explicit operator bool() const { return p_ != nullptr; }
private:
    T* p_{nullptr};
};
#endif

bool AudioCapture::start(Config config) {
    if (capturing_.load()) {
        return true;
    }
    config_ = config;
#if defined(_WIN32)
    // Initialize COM on this thread (we are called from the main thread).
    HRESULT hrInit = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    const bool comInitialized =
        SUCCEEDED(hrInit) || hrInit == RPC_E_CHANGED_MODE ||
        hrInit == S_FALSE;

    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CoCreateInstance(CLSID_MMDeviceEnumerator, nullptr,
                                  CLSCTX_ALL, IID_PPV_ARGS(enumerator.put()));
    if (FAILED(hr)) {
        if (comInitialized && hrInit == S_FALSE) CoUninitialize();
        return false;
    }

    ComPtr<IMMDevice> device;
    if (config_.deviceIndex >= 0) {
        // Enumerate and pick the index-th capture device.
        ComPtr<IMMDeviceCollection> collection;
        if (FAILED(enumerator->EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE,
                                                  collection.put()))) {
            return false;
        }
        UINT count = 0;
        collection->GetCount(&count);
        if (static_cast<UINT>(config_.deviceIndex) >= count) {
            return false;
        }
        if (FAILED(collection->Item(static_cast<UINT>(config_.deviceIndex),
                                    device.put()))) {
            return false;
        }
    } else {
        if (FAILED(enumerator->GetDefaultAudioEndpoint(eCapture, eConsole,
                                                     device.put()))) {
            return false;
        }
    }

    ComPtr<IAudioClient> audioClient;
    if (FAILED(device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr,
                                reinterpret_cast<void**>(audioClient.put())))) {
        return false;
    }

    // Request 16kHz mono Float32; let WASAPI convert from the device format.
    WAVEFORMATEX fmt{};
    fmt.wFormatTag = WAVE_FORMAT_IEEE_FLOAT;
    fmt.nChannels = static_cast<WORD>(config_.channels);
    fmt.nSamplesPerSec = config_.sampleRate;
    fmt.wBitsPerSample = 32;
    fmt.nBlockAlign = fmt.nChannels * (fmt.wBitsPerSample / 8);
    fmt.nAvgBytesPerSec = fmt.nSamplesPerSec * fmt.nBlockAlign;
    fmt.cbSize = 0;

    // AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM lets the system mix/resample from
    // the device's native format into our requested format.
    const DWORD flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM;
    const REFERENCE_TIME bufferDuration =
        static_cast<REFERENCE_TIME>(config_.bufferDurationMs) * 10000;  // ms -> 100ns
    if (FAILED(audioClient->Initialize(AUDCLNT_SHAREMODE_SHARED, flags,
                                       bufferDuration, 0, &fmt, nullptr))) {
        return false;
    }

    ComPtr<IAudioCaptureClient> captureClient;
    if (FAILED(audioClient->GetService(
            __uuidof(IAudioCaptureClient),
            reinterpret_cast<void**>(captureClient.put())))) {
        return false;
    }
    if (FAILED(audioClient->Start())) {
        return false;
    }

    device_ = device.release();
    audioClient_ = audioClient.release();
    captureClient_ = captureClient.release();
    stopRequested_.store(false);
    capturing_.store(true);
    thread_ = std::thread([this]() { captureThread(); });
    return true;
#else
    (void)config;
    return false;
#endif
}

void AudioCapture::stop() {
    if (!capturing_.load()) {
        return;
    }
    stopRequested_.store(true);
    // If stop() is called from the capture thread (e.g., via the
    // silence detector callback), don't try to join ourselves.
    // The thread will see stopRequested_ and exit on its own.
    if (thread_.get_id() == std::this_thread::get_id()) {
        return;
    }
    if (thread_.joinable()) {
        thread_.join();
    }
#if defined(_WIN32)
    if (audioClient_ != nullptr) {
        static_cast<IAudioClient*>(audioClient_)->Stop();
        static_cast<IAudioClient*>(audioClient_)->Release();
        audioClient_ = nullptr;
    }
    if (captureClient_ != nullptr) {
        static_cast<IAudioCaptureClient*>(captureClient_)->Release();
        captureClient_ = nullptr;
    }
    if (device_ != nullptr) {
        static_cast<IMMDevice*>(device_)->Release();
        device_ = nullptr;
    }
#endif
    capturing_.store(false);
}

std::vector<float> AudioCapture::getBuffer() {
    std::lock_guard<std::mutex> lk(bufferMutex_);
    return buffer_;
}

void AudioCapture::clearBuffer() {
    std::lock_guard<std::mutex> lk(bufferMutex_);
    buffer_.clear();
}

std::vector<std::wstring> AudioCapture::enumerateDevices() {
    std::vector<std::wstring> result;
#if defined(_WIN32)
    CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    ComPtr<IMMDeviceEnumerator> enumerator;
    if (FAILED(CoCreateInstance(CLSID_MMDeviceEnumerator, nullptr, CLSCTX_ALL,
                                IID_PPV_ARGS(enumerator.put())))) {
        return result;
    }
    ComPtr<IMMDeviceCollection> collection;
    if (FAILED(enumerator->EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE,
                                              collection.put()))) {
        return result;
    }
    UINT count = 0;
    collection->GetCount(&count);
    for (UINT i = 0; i < count; ++i) {
        ComPtr<IMMDevice> dev;
        if (FAILED(collection->Item(i, dev.put()))) continue;
        ComPtr<IPropertyStore> props;
        if (FAILED(dev->OpenPropertyStore(STGM_READ, props.put()))) continue;
        PROPVARIANT name;
        PropVariantInit(&name);
        if (SUCCEEDED(props->GetValue(PKEY_Device_FriendlyName, &name)) &&
            name.vt == VT_LPWSTR && name.pwszVal != nullptr) {
            result.emplace_back(name.pwszVal);
        }
        PropVariantClear(&name);
    }
    CoUninitialize();
#endif
    return result;
}

void AudioCapture::captureThread() {
#if defined(_WIN32)
    if (captureClient_ == nullptr) {
        capturing_.store(false);
        return;
    }
    auto* cap = static_cast<IAudioCaptureClient*>(captureClient_);
    while (!stopRequested_.load()) {
        Sleep(10);
        UINT32 packetLength = 0;
        HRESULT hr = cap->GetNextPacketSize(&packetLength);
        if (FAILED(hr) || packetLength == 0) {
            continue;
        }
        while (packetLength > 0) {
            BYTE* data = nullptr;
            UINT32 numFrames = 0;
            DWORD flags = 0;
            UINT64 devPos = 0, qpcPos = 0;
            hr = cap->GetBuffer(&data, &numFrames, &flags, &devPos, &qpcPos);
            if (FAILED(hr)) break;
            if (data != nullptr && numFrames > 0) {
                const float* samples = reinterpret_cast<const float*>(data);
                accumulate(samples, numFrames);
            }
            cap->ReleaseBuffer(numFrames);
            hr = cap->GetNextPacketSize(&packetLength);
            if (FAILED(hr)) break;
        }
    }
    // If stop() was called from this thread, it returned early without
    // releasing the COM objects. Clean them up here before exiting.
    if (captureClient_ != nullptr) {
        static_cast<IAudioCaptureClient*>(captureClient_)->Release();
        captureClient_ = nullptr;
    }
    if (audioClient_ != nullptr) {
        static_cast<IAudioClient*>(audioClient_)->Stop();
        static_cast<IAudioClient*>(audioClient_)->Release();
        audioClient_ = nullptr;
    }
    if (device_ != nullptr) {
        static_cast<IMMDevice*>(device_)->Release();
        device_ = nullptr;
    }
    capturing_.store(false);
#endif
}

void AudioCapture::accumulate(const float* data, std::size_t count) {
    if (count == 0) return;
    {
        std::lock_guard<std::mutex> lk(bufferMutex_);
        buffer_.insert(buffer_.end(), data, data + count);
    }
    if (onAudioData) {
        onAudioData(data, count);
    }
    // Compute RMS for the level callback.
    double sumSq = 0.0;
    for (std::size_t i = 0; i < count; ++i) {
        const double v = static_cast<double>(data[i]);
        sumSq += v * v;
    }
    const float rms = static_cast<float>(std::sqrt(sumSq / static_cast<double>(count)));
    if (onAudioLevel) {
        onAudioLevel(rms);
    }
}

}  // namespace vocawin
