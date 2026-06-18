#include <cassert>
#include <cstdint>
#include <filesystem>
#include <fstream>

#include "audio/SoundFeedback.h"

int main() {
    // 1. Default ctor: disabled by default.
    {
        vocawin::SoundFeedback sf;
        assert(!sf.isEnabled());
    }

    // 2. Enable / disable toggle.
    {
        vocawin::SoundFeedback sf;
        sf.setEnabled(true);
        assert(sf.isEnabled());
        sf.setEnabled(false);
        assert(!sf.isEnabled());
    }

    // 3. Disabled play is a no-op (returns true) on all platforms.
    {
        vocawin::SoundFeedback sf;
        sf.setEnabled(false);
        assert(sf.play(vocawin::SoundFeedback::Cue::Start));
        assert(sf.play(vocawin::SoundFeedback::Cue::Stop));
        assert(sf.play(vocawin::SoundFeedback::Cue::Error));
    }

    // 4. Missing WAV file: returns false.
    {
        const std::filesystem::path dir = "build/test-sound-feedback";
        std::filesystem::remove_all(dir);
        vocawin::SoundFeedback sf(dir);
        sf.setEnabled(true);
        assert(!sf.play(vocawin::SoundFeedback::Cue::Start));
    }

    // 5. pathFor resolves each cue to a well-known file name (sanity).
    {
        const std::filesystem::path dir = "build/test-sound-feedback-paths";
        std::filesystem::remove_all(dir);
        vocawin::SoundFeedback sf(dir);
        assert(sf.pathFor(vocawin::SoundFeedback::Cue::Start).filename() ==
               "start.wav");
        assert(sf.pathFor(vocawin::SoundFeedback::Cue::Stop).filename() ==
               "stop.wav");
        assert(sf.pathFor(vocawin::SoundFeedback::Cue::Error).filename() ==
               "error.wav");
    }

    // 6. When the WAV file exists and is non-empty, play may succeed on Win32
    //    (depends on audio device being present; on headless CI it returns
    //    false, which is also acceptable). The test only asserts no crash.
    {
        const std::filesystem::path dir = "build/test-sound-feedback-play";
        std::filesystem::remove_all(dir);
        std::filesystem::create_directories(dir);
        // Write a minimal valid WAV file header (44 bytes + 0 data).
        const std::filesystem::path wav = dir / "start.wav";
        std::ofstream out(wav, std::ios::binary);
        const std::uint8_t header[44] = {
            'R','I','F','F',  0x24,0x00,0x00,0x00,  'W','A','V','E',
            'f','m','t',' ',  0x10,0x00,0x00,0x00,  0x01,0x00,0x01,0x00,
            0x80,0x3E,0x00,0x00,  0x00,0x7D,0x00,0x00,
            0x02,0x00,0x10,0x00,  'd','a','t','a',  0x00,0x00,0x00,0x00
        };
        out.write(reinterpret_cast<const char*>(header), sizeof(header));
        out.close();
        vocawin::SoundFeedback sf(dir);
        sf.setEnabled(true);
        const bool ok = sf.play(vocawin::SoundFeedback::Cue::Start);
        // No assertion on ok - depends on audio device availability.
        (void)ok;
        std::filesystem::remove_all(dir);
    }

    return 0;
}
