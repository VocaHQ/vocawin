#include <cassert>
#include <string>

#include "input/ClipboardManager.h"

int main() {
    // 1. Default-constructed, has no saved data.
    {
        vocawin::ClipboardManager cm;
        assert(!cm.hasSavedData());
    }

    // 2. Non-Win32 stub: save returns false.
#if !defined(_WIN32)
    {
        vocawin::ClipboardManager cm;
        assert(!cm.save());
    }
#endif

    // 3. setText is callable and returns true on Win32, false on non-Win32.
    {
        vocawin::ClipboardManager cm;
        const bool ok = cm.setText(L"hello world");
#if defined(_WIN32)
        assert(ok);
        // hasSavedData is unaffected by setText.
        assert(!cm.hasSavedData());
#else
        assert(!ok);
#endif
    }

    // 4. Win32 save+restore roundtrip (when available).
#if defined(_WIN32)
    {
        // Put something on the clipboard first.
        vocawin::ClipboardManager setup;
        assert(setup.setText(L"original data"));
        // Take a fresh manager and save the current clipboard state.
        vocawin::ClipboardManager saver;
        assert(saver.save());
        assert(saver.hasSavedData());
        // Mutate the clipboard.
        vocawin::ClipboardManager mutator;
        assert(mutator.setText(L"new data"));
        // Restore.
        saver.restore();
        // The OS clipboard should now hold the saved data again.
        // (We don't introspect the clipboard here; just assert no crash.)
    }
#endif

    // 5. Restore on an empty manager is a no-op (no crash).
    {
        vocawin::ClipboardManager cm;
        cm.restore();  // should not crash
        assert(!cm.hasSavedData());
    }

    // 6. setText with empty string.
    {
        vocawin::ClipboardManager cm;
        const bool ok = cm.setText(L"");
#if defined(_WIN32)
        // Empty string is still a valid CF_UNICODETEXT set.
        assert(ok);
#else
        assert(!ok);
#endif
    }

    return 0;
}
