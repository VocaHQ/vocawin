#include <cassert>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

#include "input/TextInjector.h"

int main() {
    // 1. buildUnicodeEvents: ASCII produces 2 events per char (down + up).
    {
        const auto events = vocawin::TextInjector::buildUnicodeEvents(L"Hi");
        assert(events.size() == 4);
        assert(events[0].first == 0x0048);
        assert(events[0].second == false);
        assert(events[1].first == 0x0048);
        assert(events[1].second == true);
        assert(events[2].first == 0x0069);
        assert(events[2].second == false);
        assert(events[3].first == 0x0069);
        assert(events[3].second == true);
    }

    // 2. buildUnicodeEvents: empty string yields no events.
    {
        const auto events = vocawin::TextInjector::buildUnicodeEvents(L"");
        assert(events.empty());
    }

    // 3. buildUnicodeEvents: BMP non-ASCII (Latin-1 supplement).
    {
        const auto events = vocawin::TextInjector::buildUnicodeEvents(L"\u00E9");
        assert(events.size() == 2);
        assert(events[0].first == 0x00E9);
        assert(events[1].first == 0x00E9);
        assert(events[1].second == true);
    }

    // 4. buildUnicodeEvents: non-BMP. Windows wchar_t is UTF-16 (surrogate
    //    pair → 4 events); on macOS/Linux wchar_t is typically UTF-32
    //    (one code point → 2 events, truncated to uint16_t).
    {
        const std::wstring emojiStr = L"\U0001F600";
        const auto events = vocawin::TextInjector::buildUnicodeEvents(emojiStr);
        if (sizeof(wchar_t) == 2) {
            assert(events.size() == 4);
            assert(events[0].first == 0xD83D);
            assert(events[0].second == false);
            assert(events[1].first == 0xD83D);
            assert(events[1].second == true);
            assert(events[2].first == 0xDE00);
            assert(events[2].second == false);
            assert(events[3].first == 0xDE00);
            assert(events[3].second == true);
        } else {
            assert(events.size() == 2);
            assert(events[0].second == false);
            assert(events[1].second == true);
        }
    }

    // 5. Inject an empty string: returns true (nothing to do, not an error).
    {
        vocawin::TextInjector ti;
        assert(ti.inject(L""));
    }

    // 6. Non-Win32 stub: inject returns false.
#if !defined(_WIN32)
    {
        vocawin::TextInjector ti;
        assert(!ti.inject(L"hello"));
    }
#endif

    // 7. Configurable method and delays.
    {
        vocawin::TextInjector ti(vocawin::TextInjector::Config{
            true, vocawin::TextInjector::Method::ClipboardPaste, 200, 3000});
        const bool ok = ti.inject(L"x");
#if defined(_WIN32)
        (void)ok;
#else
        assert(!ok);
#endif
    }

    // 8. Default ctor uses sensible defaults.
    {
        vocawin::TextInjector ti;
        assert(ti.inject(L""));
    }

    return 0;
}
