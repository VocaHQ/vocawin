#include <cassert>

#include "platform/Notification.h"

int main() {
    using namespace vocawin;

    // 1. Default-constructed: not enabled.
    Notifier n;
    assert(!n.isEnabled());

    // 2. Disabled path: show() is a silent no-op on every platform,
    //    including Win32 (where the enabled path would block on a
    //    MessageBoxW dialog in headless CI).
    assert(n.show("Test title", "Test body"));
    assert(n.show(std::wstring(L"Test title"), std::wstring(L"Test body")));

    // 3. setEnabled toggles state.
    n.setEnabled(true);
    assert(n.isEnabled());
    n.setEnabled(false);
    assert(!n.isEnabled());

    // 4. Disabled after enable still no-ops.
    assert(!n.isEnabled());
    assert(n.show("title", "body"));

    return 0;
}
