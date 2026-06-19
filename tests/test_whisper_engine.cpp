#include <cassert>
#include <filesystem>
#include <optional>
#include <string>
#include <vector>

#include "speech/WhisperEngine.h"

int main() {
    // 1. Default-constructed engine is not loaded.
    {
        vocawin::WhisperEngine eng;
        assert(!eng.isModelLoaded());
    }

    // 2. Transcribe without a loaded model returns nullopt.
    {
        vocawin::WhisperEngine eng;
        std::vector<float> audio(16000, 0.0f);
        const auto r = eng.transcribe(audio);
        assert(!r.has_value());
    }

    // 3. Transcribe empty audio returns nullopt.
    {
        vocawin::WhisperEngine eng;
        std::vector<float> audio;
        const auto r = eng.transcribe(audio);
        assert(!r.has_value());
    }

    // 4. loadModel with a non-existent file returns false.
    {
        vocawin::WhisperEngine eng;
        const std::filesystem::path missing = "build/test-whisper-engine/missing.bin";
        std::filesystem::remove(missing);
        assert(!eng.loadModel(missing, vocawin::WhisperEngine::GpuBackend{"CPU", ""}, 1));
        assert(!eng.isModelLoaded());
    }

    // 5. unloadModel without prior load is a no-op (no crash).
    {
        vocawin::WhisperEngine eng;
        eng.unloadModel();
        assert(!eng.isModelLoaded());
    }

    // 6. setLanguage and setTranslateMode don't crash.
    {
        vocawin::WhisperEngine eng;
        eng.setLanguage("en");
        eng.setLanguage("auto");
        eng.setTranslateMode(true);
        eng.setTranslateMode(false);
    }

    // 7. filterText: hallucination markers removed.
    {
        const std::wstring raw = L"Hello world [BLANK_AUDIO]";
        const auto filtered = vocawin::WhisperEngine::filterText(raw);
        assert(filtered == L"Hello world");
    }

    // 8. filterText: angle-bracket tags removed.
    {
        const std::wstring raw = L"<minimal>the quick brown fox";
        const auto filtered = vocawin::WhisperEngine::filterText(raw);
        assert(filtered == L"the quick brown fox");
    }

    // 9. filterText: collapses multiple spaces.
    {
        const std::wstring raw = L"hello   world";
        const auto filtered = vocawin::WhisperEngine::filterText(raw);
        assert(filtered == L"hello world");
    }

    // 10. filterText: trims whitespace.
    {
        const std::wstring raw = L"   hello   ";
        const auto filtered = vocawin::WhisperEngine::filterText(raw);
        assert(filtered == L"hello");
    }

    // 11. filterText: only markers -> empty.
    {
        const std::wstring raw = L"[BLANK_AUDIO] [NO_SPEECH] <minimal>";
        const auto filtered = vocawin::WhisperEngine::filterText(raw);
        assert(filtered.empty());
    }

    // 12. filterText: multiple known markers + surrounding text.
    {
        const std::wstring raw = L"  [Music]  real content  [silence]  ";
        const auto filtered = vocawin::WhisperEngine::filterText(raw);
        assert(filtered == L"real content");
    }

    // 13. Destructor on a non-loaded engine does not crash.
    {
        vocawin::WhisperEngine eng;
    }

    return 0;
}
