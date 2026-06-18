#include <cassert>
#include <string>

#include "ui/TrayIcon.h"

int main() {
    // 1. Initialize + shutdown without crash (existing scenario).
    {
        vocawin::TrayIcon tray;
        assert(tray.initialize());
        tray.shutdown();
    }

    // 2. Default state is Idle.
    {
        vocawin::TrayIcon tray;
        assert(tray.state() == vocawin::TrayIcon::State::Idle);
    }

    // 3. setState transitions update internal state.
    {
        vocawin::TrayIcon tray;
        assert(tray.initialize());
        assert(tray.setState(vocawin::TrayIcon::State::Recording));
        assert(tray.state() == vocawin::TrayIcon::State::Recording);
        assert(tray.setState(vocawin::TrayIcon::State::Processing));
        assert(tray.state() == vocawin::TrayIcon::State::Processing);
        assert(tray.setState(vocawin::TrayIcon::State::Error));
        assert(tray.state() == vocawin::TrayIcon::State::Error);
        assert(tray.setState(vocawin::TrayIcon::State::NoModel));
        assert(tray.state() == vocawin::TrayIcon::State::NoModel);
        assert(tray.setState(vocawin::TrayIcon::State::Idle));
        assert(tray.state() == vocawin::TrayIcon::State::Idle);
    }

    // 4. setState before initialize returns false.
    {
        vocawin::TrayIcon tray;
        assert(!tray.setState(vocawin::TrayIcon::State::Recording));
    }

    // 5. setTooltip does not crash.
    {
        vocawin::TrayIcon tray;
        assert(tray.initialize());
        tray.setTooltip(L"Custom tooltip");
    }

    // 6. defaultTooltipFor returns non-empty strings for every state.
    {
        const auto states = {
            vocawin::TrayIcon::State::Idle,
            vocawin::TrayIcon::State::Recording,
            vocawin::TrayIcon::State::Processing,
            vocawin::TrayIcon::State::Error,
            vocawin::TrayIcon::State::NoModel,
        };
        for (const auto s : states) {
            const auto tip = vocawin::TrayIcon::defaultTooltipFor(s);
            assert(!tip.empty());
        }
    }

    // 7. Shutdown after setState works (no resource leak).
    {
        vocawin::TrayIcon tray;
        assert(tray.initialize());
        tray.setState(vocawin::TrayIcon::State::Processing);
        tray.shutdown();
    }

    // 8. setState after shutdown returns false.
    {
        vocawin::TrayIcon tray;
        assert(tray.initialize());
        tray.shutdown();
        assert(!tray.setState(vocawin::TrayIcon::State::Recording));
    }

    return 0;
}
