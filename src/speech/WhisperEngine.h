#pragma once

#include <filesystem>
#include <optional>
#include <string>
#include <vector>

namespace vocawin {

// Thin wrapper around whisper.cpp's C API. Owns a single whisper_context
// at a time. Safe to construct/destroy without loading a model. The MVP
// uses CPU inference (whisper.cpp is built with all GPU backends off).
//
// Per SPEC \u00a74.2.3 (page ~325). Hallucination filter `filterText` removes
// common Whisper artifacts (BLANK_AUDIO, NO_SPEECH, Music, etc.) and
// collapses whitespace.
class WhisperEngine {
public:
    struct Result {
        std::wstring text;
        std::string language;     // e.g. "en", or "auto" if unknown
        float confidence{0.0f};   // Avg token probability, 0..1
    };

    WhisperEngine();
    ~WhisperEngine();

    WhisperEngine(const WhisperEngine&) = delete;
    WhisperEngine& operator=(const WhisperEngine&) = delete;

    // Load a GGML model from disk. Frees any previously-loaded model.
    // Returns true on success.
    bool loadModel(const std::filesystem::path& modelPath, int nThreads = 4);

    // Free the currently-loaded model (if any). Safe to call when not loaded.
    void unloadModel();

    bool isModelLoaded() const;

    void setLanguage(const std::string& lang);  // "auto", "en", etc.
    void setTranslateMode(bool translate);      // Translate to English

    // Transcribe 16kHz mono Float32 PCM. Returns nullopt on failure (no
    // model, empty audio, inference error).
    std::optional<Result> transcribe(const std::vector<float>& audioData);

    // Strip hallucination markers, angle-bracket tags, and collapse
    // whitespace. Public so callers can post-process or unit-test in
    // isolation. Pure function (no model needed).
    static std::wstring filterText(const std::wstring& raw);

private:
    struct Impl;
    Impl* impl_;
};

}  // namespace vocawin
