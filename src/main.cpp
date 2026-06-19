#include <iostream>
#include <string>

#if defined(_WIN32)
#include <windows.h>
#endif

#include "app/AppController.h"

namespace {

void printUsage() {
    std::cout << "VocaWin \u2014 Offline voice-to-text for Windows\n"
              << "Usage: vocawin [--minimized]\n"
              << "  --minimized  Start with no console output (tray only)\n"
              << "  --help       Show this help and exit\n";
}

bool wantsHelp(int argc, char** argv) {
    for (int i = 1; i < argc; ++i) {
        const std::string a = argv[i];
        if (a == "--help" || a == "-h" || a == "/?") {
            return true;
        }
    }
    return false;
}

}  // namespace

int main(int argc, char* argv[]) {
    if (wantsHelp(argc, argv)) {
        printUsage();
        return 0;
    }

    vocawin::AppController app;
    if (!app.initialize()) {
        std::cerr << "Failed to initialize VocaWin\n";
        return 1;
    }

    std::cout << "VocaWin initialized. State=" << static_cast<int>(app.state()) << "\n";

#if defined(_WIN32)
    app.onQuitRequested = []() { PostQuitMessage(0); };
    MSG msg;
    while (GetMessage(&msg, nullptr, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
        if (msg.message == WM_QUIT) {
            break;
        }
    }
    (void)app.settingsWindow();
#else
    std::cout << "Press Enter to shut down.\n";
    std::cin.get();
#endif

    app.shutdown();
    return 0;
}
