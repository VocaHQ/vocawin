#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace vocawin {

// Deep-copy clipboard save/restore, plus a text-set helper used by the
// TextInjector fallback path. Windows-only at runtime; non-Win32 builds
// provide safe no-op stubs so the rest of the app can compile and link.
//
// Per SPEC \u00a74.2.7 (page ~540).
class ClipboardManager {
public:
    ClipboardManager();
    ~ClipboardManager();

    ClipboardManager(const ClipboardManager&) = delete;
    ClipboardManager& operator=(const ClipboardManager&) = delete;
    ClipboardManager(ClipboardManager&& other) noexcept;
    ClipboardManager& operator=(ClipboardManager&& other) noexcept;

    // Deep-copy every clipboard format currently on the clipboard. Returns
    // true on success (at least one format was captured).
    bool save();

    // Restore previously-saved clipboard contents. No-op if save() was not
    // called (or failed). Safe to call multiple times.
    void restore();

    // Replace clipboard contents with the given Unicode text (CF_UNICODETEXT).
    // Returns true on success.
    bool setText(const std::wstring& text);

    // True if save() succeeded and the data has not been restored yet.
    bool hasSavedData() const;

private:
    struct ClipboardEntry {
        std::uint32_t format;
        std::vector<std::uint8_t> data;
    };

    void clearSaved();

    std::vector<ClipboardEntry> saved_;
    bool hasSaved_{false};
#if defined(_WIN32)
    void* hwnd_{nullptr};  // HWND used as the clipboard owner for restore
#endif
};

}  // namespace vocawin
