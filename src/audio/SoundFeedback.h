#pragma once

#include <filesystem>

namespace vocawin {

// Plays short WAV cues for recording start/stop/error feedback. Resolves
// a per-cue filename under the configured sounds directory and delegates
// playback to the platform audio API. On non-Win32 platforms `play` is
// a no-op that returns false.
//
// Per SPEC \u00a74.1 (page ~175) and \u00a75.1 (page ~590).
class SoundFeedback {
public:
    enum class Cue { Start, Stop, Error };

    explicit SoundFeedback(std::filesystem::path soundsDir = {});

    void setEnabled(bool enabled) { enabled_ = enabled; }
    bool isEnabled() const { return enabled_; }

    // Play the WAV file for the given cue. Returns true on success, false
    // if the file is missing, disabled, or the platform cannot play it.
    bool play(Cue cue);

    // Resolve the filesystem path for a cue (used by AppController and by
    // the bundle/sounds generator). Exposed for testability.
    std::filesystem::path pathFor(Cue cue) const;

private:
    std::filesystem::path soundsDir_;
    bool enabled_{false};
};

}  // namespace vocawin
