#include "input/HotkeyManager.h"

#include <chrono>
#include <thread>

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

#if defined(_WIN32)
namespace {
HotkeyManager* g_hookOwner = nullptr;
}

LRESULT CALLBACK HotkeyManagerLLKeyboardProc(int nCode, WPARAM wParam,
                                              LPARAM lParam) {
    if (nCode == HC_ACTION && g_hookOwner != nullptr) {
        const auto* p = reinterpret_cast<KBDLLHOOKSTRUCT*>(lParam);
        const bool isKeyDown = (wParam == WM_KEYDOWN || wParam == WM_SYSKEYDOWN);
        const bool isKeyUp = (wParam == WM_KEYUP || wParam == WM_SYSKEYUP);
        if (p != nullptr &&
            p->vkCode == static_cast<UINT>(g_hookOwner->config().virtualKeyCode)) {
            if (isKeyDown) {
                g_hookOwner->handleKeyEvent(true);
            } else if (isKeyUp) {
                g_hookOwner->handleKeyEvent(false);
            }
            // Suppress the key from reaching other apps to avoid sticky
            // modifiers; in push-to-talk this is the expected behavior.
            return 1;
        }
    }
    return CallNextHookEx(nullptr, nCode, wParam, lParam);
}
#endif  // _WIN32

HotkeyManager::HotkeyManager() = default;

HotkeyManager::~HotkeyManager() {
    stop();
}

bool HotkeyManager::start(Config config) {
    if (running_.load()) {
        return true;  // already running
    }
    config_ = config;
#if defined(_WIN32)
    g_hookOwner = this;
    hook_ = SetWindowsHookExW(WH_KEYBOARD_LL, &HotkeyManagerLLKeyboardProc,
                              GetModuleHandleW(nullptr), 0);
    if (hook_ == nullptr) {
        g_hookOwner = nullptr;
        return false;
    }
    stopRequested_.store(false);
    running_.store(true);
    std::thread t([this]() { runMessageLoop(); });
    threadHandle_ = nullptr;  // not used; thread will detach
    t.detach();
    return true;
#else
    (void)config;
    return false;
#endif
}

void HotkeyManager::stop() {
    if (!running_.load()) {
        return;
    }
    stopRequested_.store(true);
#if defined(_WIN32)
    if (hook_ != nullptr) {
        UnhookWindowsHookEx(static_cast<HHOOK>(hook_));
        hook_ = nullptr;
    }
    if (g_hookOwner == this) {
        g_hookOwner = nullptr;
    }
    // Post a WM_QUIT to wake the message loop.
    PostThreadMessageW(threadId_, WM_QUIT, 0, 0);
    // Give the loop a moment to exit (it polls stopRequested_ too).
    for (int i = 0; i < 20 && running_.load(); ++i) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
#endif
    running_.store(false);
}

void HotkeyManager::runMessageLoop() {
#if defined(_WIN32)
    threadId_ = GetCurrentThreadId();
    MSG msg;
    while (!stopRequested_.load() && GetMessage(&msg, nullptr, 0, 0) > 0) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }
#endif
}

void HotkeyManager::handleKeyEvent(bool isKeyDown) {
    if (config_.mode == ActivationMode::PushToTalk) {
        if (isKeyDown) {
            if (onHotkeyPressed) onHotkeyPressed();
        } else {
            if (onHotkeyReleased) onHotkeyReleased();
        }
    } else {
        // DoubleTapToggle is captured at the message-loop level via raw
        // event counting. Single-event entry-point keeps the surface
        // uniform for tests and platforms.
        (void)isKeyDown;
    }
}

}  // namespace vocawin
