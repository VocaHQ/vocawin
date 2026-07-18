#include "speech/WhisperEngine.h"

#include <cstdint>
#include <cstdio>
#include <regex>
#include <string>

#include "whisper.h"

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

struct WhisperEngine::Impl {
    whisper_context* ctx{nullptr};
    std::string language{"auto"};
    bool translate{false};
    int nThreads{4};
    GpuBackend backend{"CPU", "Generic CPU"};
};

namespace {

std::wstring utf8ToWstring(const std::string& s) {
    if (s.empty()) return {};
#if defined(_WIN32)
    const int needed = MultiByteToWideChar(CP_UTF8, 0, s.c_str(),
                                           static_cast<int>(s.size()),
                                           nullptr, 0);
    if (needed <= 0) return {};
    std::wstring out(static_cast<std::size_t>(needed), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.c_str(), static_cast<int>(s.size()),
                        &out[0], needed);
    return out;
#else
    std::wstring out;
    out.reserve(s.size());
    for (std::size_t i = 0; i < s.size(); ) {
        unsigned char c = static_cast<unsigned char>(s[i]);
        if (c < 0x80) {
            out.push_back(static_cast<wchar_t>(c));
            ++i;
        } else {
            // ASCII-only fallback: production path is Win32 (UTF-16).
            out.push_back(L'?');
            ++i;
        }
    }
    return out;
#endif
}

}  // namespace

WhisperEngine::WhisperEngine() : impl_(new Impl()) {}

WhisperEngine::~WhisperEngine() {
    unloadModel();
    delete impl_;
}

bool WhisperEngine::isModelLoaded() const {
    return impl_->ctx != nullptr;
}

bool WhisperEngine::loadModel(const std::filesystem::path& modelPath,
                              const GpuBackend& gpu,
                              int nThreads) {
    unloadModel();
    if (!std::filesystem::exists(modelPath)) {
        return false;
    }
    impl_->backend = gpu;
    whisper_context_params cparams = whisper_context_default_params();
    cparams.use_gpu = (gpu.name == "CUDA" || gpu.name == "Vulkan");
    cparams.flash_attn = false;
    impl_->ctx = whisper_init_from_file_with_params(
        modelPath.string().c_str(), cparams);
    if (impl_->ctx == nullptr) {
        return false;
    }
    impl_->nThreads = nThreads > 0 ? nThreads : 1;
    return true;
}

void WhisperEngine::unloadModel() {
    if (impl_->ctx != nullptr) {
        whisper_free(impl_->ctx);
        impl_->ctx = nullptr;
    }
}

const WhisperEngine::GpuBackend& WhisperEngine::gpuBackend() const {
    return impl_->backend;
}

void WhisperEngine::setLanguage(const std::string& lang) {
    impl_->language = lang;
}

void WhisperEngine::setTranslateMode(bool translate) {
    impl_->translate = translate;
}

std::optional<WhisperEngine::Result>
WhisperEngine::transcribe(const std::vector<float>& audioData) {
    if (impl_->ctx == nullptr || audioData.empty()) {
        return std::nullopt;
    }

    whisper_full_params wparams =
        whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    wparams.n_threads = impl_->nThreads > 0 ? impl_->nThreads : 1;
    wparams.print_progress = false;
    wparams.print_realtime = false;
    wparams.print_timestamps = false;
    wparams.print_special = false;
    wparams.translate = impl_->translate;
    wparams.no_context = true;
    wparams.single_segment = false;
    if (impl_->language == "auto" || impl_->language.empty()) {
        wparams.language = nullptr;  // auto-detect
    } else {
        wparams.language = impl_->language.c_str();
    }

    const int ret = whisper_full(impl_->ctx, wparams, audioData.data(),
                                 static_cast<int>(audioData.size()));
    if (ret != 0) {
        return std::nullopt;
    }

    const int nSegments = whisper_full_n_segments(impl_->ctx);
    std::string fullText;
    float nsp = 0.0f;
    for (int i = 0; i < nSegments; ++i) {
        const char* seg = whisper_full_get_segment_text(impl_->ctx, i);
        if (seg != nullptr) {
            fullText += seg;
        }
    }
    // whisper_full_get_segment_no_speech_prob requires a valid segment
    // index; pure noise can yield zero segments — never touch index 0 then.
    if (nSegments > 0) {
        nsp = whisper_full_get_segment_no_speech_prob(impl_->ctx, 0);
    }

    Result r;
    r.text = filterText(utf8ToWstring(fullText));
    r.language = impl_->language;
    r.confidence = 1.0f - (nSegments > 0 ? nsp : 0.0f);
    return r;
}

std::wstring WhisperEngine::filterText(const std::wstring& raw) {
    if (raw.empty()) {
        return {};
    }
    std::wstring s = raw;

    static const std::wregex kBracketMarker(
        L"\\[(BLANK_AUDIO|NO_SPEECH|Music|Silence|music|silence|bing|thud|"
        L"clatter|background|noise|rustle|clears|throat|blank_audio|"
        L"no_speech|INAUDIBLE|inaudible)\\]");
    s = std::regex_replace(s, kBracketMarker, L"");

    static const std::wregex kAngleTags(L"<[^>]+>");
    s = std::regex_replace(s, kAngleTags, L"");

    // Collapse multiple whitespace to single space, trim.
    static const std::wregex kMultiSpace(L"\\s+");
    s = std::regex_replace(s, kMultiSpace, L" ");

    auto notSpace = [](wchar_t c) { return !std::iswspace(static_cast<wint_t>(c)); };
    const auto first = std::find_if(s.begin(), s.end(), notSpace);
    if (first == s.end()) {
        return {};
    }
    const auto last = std::find_if(s.rbegin(), s.rend(), notSpace).base();
    return std::wstring(first, last);
}

}  // namespace vocawin
