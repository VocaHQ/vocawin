#include <cassert>

#include "ui/SettingsWindow.h"

int main() {
    vocawin::SettingsWindow w;

    // 1. Default-constructed: not visible.
    assert(!w.isVisible());

    // 2. setLoadHandler is required for show() to succeed.
    bool loadCalled = false;
    w.setLoadHandler([&loadCalled]() {
        loadCalled = true;
        vocawin::Settings s;
        return s;
    });

    // 3. setSaveHandler registers cleanly.
    bool saveCalled = false;
    w.setSaveHandler([&saveCalled](const vocawin::Settings&) {
        saveCalled = true;
        return true;
    });

    // 4. setRecommendHandler + setSystemInfoHandler + setOnAboutHandler.
    w.setRecommendHandler([](std::size_t, std::size_t, bool) {
        return std::string("base.en");
    });
    w.setSystemInfoHandler([]() { return std::wstring(L"Test system"); });
    w.setOnAboutHandler([]() { return std::wstring(L"Test about"); });

    // 5. show() on non-Win32 returns false (the window is a no-op).
    //    On Win32 it returns true and creates the actual dialog.
#if !defined(_WIN32)
    const bool shown = w.show();
    assert(!shown);
    assert(!w.isVisible());
#endif

    // 6. pumpMessage returns false when not visible.
    assert(!w.pumpMessage());

    // 7. Multiple show() calls on non-Win32 are idempotent.
#if !defined(_WIN32)
    assert(!w.show());
    assert(!w.show());
#endif

    // 8. hide() is safe to call when not visible.
    w.hide();
    assert(!w.isVisible());

    return 0;
}
