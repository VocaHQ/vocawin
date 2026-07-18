#include <cassert>
#include <chrono>
#include <cstdint>
#include <thread>

#include "input/HotkeyManager.h"

int main() {
    // 1. Default config matches spec (VK_RCONTROL, push-to-talk, 400ms).
    {
        vocawin::HotkeyManager::Config cfg;
        assert(cfg.virtualKeyCode == 0xA3);  // VK_RCONTROL
        assert(cfg.mode ==
               vocawin::HotkeyManager::ActivationMode::PushToTalk);
        assert(cfg.doubleTapThresholdMs == 400.0);
    }

    // 2. Default-constructed manager is not running.
    {
        vocawin::HotkeyManager hk;
        assert(!hk.isRunning());
    }

    // 3. handleKeyEvent drives the same path as the LL hook (PushToTalk).
    {
        vocawin::HotkeyManager hk;
        int presses = 0;
        int releases = 0;
        hk.onHotkeyPressed = [&]() { ++presses; };
        hk.onHotkeyReleased = [&]() { ++releases; };

        hk.handleKeyEvent(true);
        assert(presses == 1);
        assert(releases == 0);

        // Auto-repeat while held must not re-fire pressed.
        hk.handleKeyEvent(true);
        hk.handleKeyEvent(true);
        assert(presses == 1);

        hk.handleKeyEvent(false);
        assert(releases == 1);

        // Spurious release ignored.
        hk.handleKeyEvent(false);
        assert(releases == 1);

        // Next press/release cycle works.
        hk.handleKeyEvent(true);
        hk.handleKeyEvent(false);
        assert(presses == 2);
        assert(releases == 2);
    }

#if !defined(_WIN32)
    // 4. Non-Win32 stub: start returns false.
    {
        vocawin::HotkeyManager hk;
        vocawin::HotkeyManager::Config cfg;
        assert(!hk.start(cfg));
        assert(!hk.isRunning());
        hk.stop();
    }
#else
    // 4. Win32: start installs hook on pump thread; stop joins cleanly.
    {
        vocawin::HotkeyManager hk;
        vocawin::HotkeyManager::Config cfg;
        cfg.virtualKeyCode = 0xA3;
        cfg.mode = vocawin::HotkeyManager::ActivationMode::PushToTalk;
        int presses = 0;
        hk.onHotkeyPressed = [&]() { ++presses; };
        hk.onHotkeyReleased = [&]() {};
        const bool started = hk.start(cfg);
        assert(hk.isRunning() == started);
        if (!started) {
            (void)hk.lastErrorCode();
        } else {
            // handleKeyEvent still works while hook is live (same entry as LL).
            hk.handleKeyEvent(true);
            hk.handleKeyEvent(false);
            assert(presses == 1);
            hk.stop();
            assert(!hk.isRunning());
        }
    }

    // 5. Re-start after stop.
    {
        vocawin::HotkeyManager hk;
        vocawin::HotkeyManager::Config cfg;
        if (hk.start(cfg)) {
            hk.stop();
            assert(hk.start(cfg));
            assert(hk.isRunning());
            hk.stop();
        }
    }
#endif

    // 6. DoubleTapToggle config is selectable.
    {
        vocawin::HotkeyManager::Config cfg;
        cfg.mode = vocawin::HotkeyManager::ActivationMode::DoubleTapToggle;
        cfg.doubleTapThresholdMs = 250.0;
        assert(cfg.mode ==
               vocawin::HotkeyManager::ActivationMode::DoubleTapToggle);
    }

    return 0;
}
