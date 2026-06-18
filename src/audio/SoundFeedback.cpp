#include "audio/SoundFeedback.h"

#if defined(_WIN32)
#include <windows.h>
#include <mmsystem.h>
#pragma comment(lib, "winmm.lib")
#endif

namespace vocawin {

SoundFeedback::SoundFeedback(std::filesystem::path soundsDir)
    : soundsDir_(std::move(soundsDir)) {
    if (soundsDir_.empty()) {
        soundsDir_ = std::filesystem::current_path() / "sounds";
    }
}

std::filesystem::path SoundFeedback::pathFor(Cue cue) const {
    const char* name = "start.wav";
    switch (cue) {
        case Cue::Start: name = "start.wav"; break;
        case Cue::Stop:  name = "stop.wav";  break;
        case Cue::Error: name = "error.wav"; break;
    }
    return soundsDir_ / name;
}

bool SoundFeedback::play(Cue cue) {
    if (!enabled_) {
        return true;  // disabled = no-op success
    }
    const auto path = pathFor(cue);
    if (!std::filesystem::exists(path) ||
        std::filesystem::file_size(path) < 44) {
        return false;
    }
#if defined(_WIN32)
    // Use PlaySound with SND_FILENAME | SND_ASYNC | SND_NODEFAULT so a
    // missing or invalid file is silent rather than a system beep.
    const BOOL ok = PlaySoundW(path.wstring().c_str(), nullptr,
                               SND_FILENAME | SND_ASYNC | SND_NODEFAULT);
    return ok != 0;
#else
    (void)path;
    return false;
#endif
}

}  // namespace vocawin
