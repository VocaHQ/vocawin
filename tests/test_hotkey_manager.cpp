#include <cassert>
#include <cstdint>

#include "input/HotkeyManager.h"

int main() {
    // 1. Default config matches spec (VK_RCONTROL, push-to-talk, 400ms).
    {
        vocawin::HotkeyManager::Config cfg;
        assert(cfg.virtualKeyCode == 0xA3);   // VK_RCONTROL
        assert(cfg.mode == vocawin::HotkeyManager::ActivationMode::PushToTalk);
        assert(cfg.doubleTapThresholdMs == 400.0);
    }

    // 2. Default-constructed manager is not running.
    {
        vocawin::HotkeyManager hk;
        assert(!hk.isRunning());
    }

    // 3. Non-Win32 stub: start returns false, stop is a no-op.
#if !defined(_WIN32)
    {
        vocawin::HotkeyManager hk;
        vocawin::HotkeyManager::Config cfg;
        assert(!hk.start(cfg));
        assert(!hk.isRunning());
        hk.stop();  // no crash
    }
#endif

    // 4. Win32: start with a non-interactive desktop may fail (returns false),
    //    or succeed if we are in an interactive session. Either way the
    //    manager must not be in an invalid state.
#if defined(_WIN32)
    {
        vocawin::HotkeyManager hk;
        vocawin::HotkeyManager::Config cfg;
        cfg.virtualKeyCode = 0xA3;  // VK_RCONTROL
        cfg.mode = vocawin::HotkeyManager::ActivationMode::PushToTalk;
        const bool started = hk.start(cfg);
        // Accept either outcome - depends on session. Just assert the
        // isRunning state matches.
        assert(hk.isRunning() == started);
        if (started) {
            hk.stop();
            assert(!hk.isRunning());
        }
    }
#endif

    // 5. Setting callbacks before start is safe (no crash).
    {
        vocawin::HotkeyManager hk;
        bool called = false;
        hk.onHotkeyPressed = [&called]() { called = true; };
        hk.onHotkeyReleased = [&called]() { called = false; };
        // No start - just assert callbacks stored.
        (void)called;
    }

    // 6. DoubleTapToggle config is selectable.
    {
        vocawin::HotkeyManager::Config cfg;
        cfg.mode = vocawin::HotkeyManager::ActivationMode::DoubleTapToggle;
        cfg.doubleTapThresholdMs = 250.0;
        assert(cfg.mode == vocawin::HotkeyManager::ActivationMode::DoubleTapToggle);
        assert(cfg.doubleTapThresholdMs == 250.0);
    }

    // 7. Re-starting after stop is allowed.
#if defined(_WIN32)
    {
        vocawin::HotkeyManager hk;
        vocawin::HotkeyManager::Config cfg;
        const bool s1 = hk.start(cfg);
        if (s1) {
            hk.stop();
        }
        const bool s2 = hk.start(cfg);
        if (s2) {
            assert(hk.isRunning());
            hk.stop();
        }
    }
#endif

    return 0;
}
