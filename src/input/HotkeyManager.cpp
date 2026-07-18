#include "input/HotkeyManager.h"

#include <chrono>

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

#if defined(_WIN32)
namespace {
HotkeyManager* g_hookOwner = nullptr;

LRESULT CALLBACK HotkeyManagerLLKeyboardProc(int nCode, WPARAM wParam,
                                              LPARAM lParam) {
    if (nCode == HC_ACTION && g_hookOwner != nullptr) {
        const auto* p = reinterpret_cast<KBDLLHOOKSTRUCT*>(lParam);
        if (p != nullptr &&
            p->vkCode ==
                static_cast<DWORD>(g_hookOwner->config().virtualKeyCode)) {
            const bool isKeyDown =
                (wParam == WM_KEYDOWN || wParam == WM_SYSKEYDOWN);
            const bool isKeyUp =
                (wParam == WM_KEYUP || wParam == WM_SYSKEYUP);
            // Keep the hook callback fast: only debounce + invoke the
            // already-wired std::function (which should PostMessage to UI).
            // Heavy work here trips LowLevelHooksTimeout and Windows
            // silently unhooks us.
            if (isKeyDown) {
                g_hookOwner->handleKeyEvent(true);
            } else if (isKeyUp) {
                g_hookOwner->handleKeyEvent(false);
            }
            // Swallow the hotkey so it does not type into the focused app.
            return 1;
        }
    }
    return CallNextHookEx(nullptr, nCode, wParam, lParam);
}
}  // namespace
#endif  // _WIN32

HotkeyManager::HotkeyManager() = default;

HotkeyManager::~HotkeyManager() {
    stop();
}

bool HotkeyManager::start(Config config) {
    std::lock_guard<std::mutex> life(lifecycleMutex_);
    // Always fully stop any prior pump (join) before clearing stopRequested_.
    // This prevents a detached ghost pump from racing install/uninstall.
    if (running_.load() || pumpThread_.joinable() || pumpAlive_.load()) {
        // Nested stop without re-taking lifecycleMutex_ — call internal path.
        stopRequested_.store(true);
#if defined(_WIN32)
        if (hook_ != nullptr) {
            UnhookWindowsHookEx(static_cast<HHOOK>(hook_));
            hook_ = nullptr;
        }
        if (g_hookOwner == this) {
            g_hookOwner = nullptr;
        }
        if (threadId_ != 0) {
            PostThreadMessageW(threadId_, WM_QUIT, 0, 0);
        }
#endif
        if (pumpThread_.joinable()) {
            pumpThread_.join();
        }
        running_.store(false);
        pumpAlive_.store(false);
        threadId_ = 0;
        keyHeld_.store(false);
    }

    config_ = config;
    lastErrorCode_.store(0);
    keyHeld_.store(false);

#if defined(_WIN32)
    const std::uint64_t gen = generation_.fetch_add(1) + 1;
    stopRequested_.store(false);
    pumpAlive_.store(true);

    std::atomic<bool> installDone{false};
    std::atomic<bool> installOk{false};

    // Install hook ON the same thread that pumps messages (MSDN requirement
    // for WH_KEYBOARD_LL). Callbacks must stay non-blocking (PostMessage).
    pumpThread_ = std::thread([this, gen, &installDone, &installOk]() {
        installHookOnPumpThread();
        installOk.store(hook_ != nullptr);
        installDone.store(true);
        if (hook_ == nullptr) {
            pumpAlive_.store(false);
            return;
        }
        runPumpThread(gen);
        uninstallHookOnPumpThread();
        if (generation_.load() == gen) {
            pumpAlive_.store(false);
        }
    });

    for (int i = 0; i < 200 && !installDone.load(); ++i) {
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    if (!installDone.load() || !installOk.load()) {
        stopRequested_.store(true);
        if (threadId_ != 0) {
            PostThreadMessageW(threadId_, WM_QUIT, 0, 0);
        }
        if (pumpThread_.joinable()) {
            pumpThread_.join();
        }
        running_.store(false);
        pumpAlive_.store(false);
        return false;
    }
    running_.store(true);
    return true;
#else
    (void)config;
    pumpAlive_.store(false);
    return false;
#endif
}

void HotkeyManager::stop() {
    std::lock_guard<std::mutex> life(lifecycleMutex_);
    if (!running_.load() && !pumpThread_.joinable() && !pumpAlive_.load()) {
        return;
    }
    stopRequested_.store(true);
    generation_.fetch_add(1);  // invalidate in-flight pump generation
#if defined(_WIN32)
    // Unhook first so no new callbacks enter handleKeyEvent.
    if (hook_ != nullptr) {
        UnhookWindowsHookEx(static_cast<HHOOK>(hook_));
        hook_ = nullptr;
    }
    if (g_hookOwner == this) {
        g_hookOwner = nullptr;
    }
    if (threadId_ != 0) {
        PostThreadMessageW(threadId_, WM_QUIT, 0, 0);
    }
#endif
    // Join is safe: wired callbacks only PostMessage (non-blocking).
    if (pumpThread_.joinable()) {
        pumpThread_.join();
    }
    hook_ = nullptr;
    threadId_ = 0;
    running_.store(false);
    pumpAlive_.store(false);
    keyHeld_.store(false);
}

void HotkeyManager::installHookOnPumpThread() {
#if defined(_WIN32)
    threadId_ = GetCurrentThreadId();
    MSG probe{};
    PeekMessageW(&probe, nullptr, WM_USER, WM_USER, PM_NOREMOVE);

    g_hookOwner = this;
    hook_ = SetWindowsHookExW(WH_KEYBOARD_LL, &HotkeyManagerLLKeyboardProc,
                              GetModuleHandleW(nullptr), 0);
    if (hook_ == nullptr) {
        lastErrorCode_.store(static_cast<std::uint32_t>(GetLastError()));
        g_hookOwner = nullptr;
    }
#endif
}

void HotkeyManager::uninstallHookOnPumpThread() {
#if defined(_WIN32)
    if (hook_ != nullptr) {
        UnhookWindowsHookEx(static_cast<HHOOK>(hook_));
        hook_ = nullptr;
    }
    if (g_hookOwner == this) {
        g_hookOwner = nullptr;
    }
#endif
}

void HotkeyManager::runPumpThread(std::uint64_t generation) {
#if defined(_WIN32)
    // Poll stopRequested_ so stop() can join promptly after unhook+WM_QUIT.
    // Wired onHotkeyPressed/Released only PostMessage — never block here.
    MSG msg;
    while (!stopRequested_.load() && generation_.load() == generation) {
        while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
            if (msg.message == WM_QUIT) {
                return;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Sleep(10);
    }
#else
    (void)generation;
#endif
}

void HotkeyManager::handleKeyEvent(bool isKeyDown) {
    if (config_.mode == ActivationMode::PushToTalk) {
        if (isKeyDown) {
            bool expected = false;
            if (!keyHeld_.compare_exchange_strong(expected, true)) {
                return;
            }
            std::function<void()> cb;
            {
                std::lock_guard<std::mutex> lk(callbackMutex_);
                cb = onHotkeyPressed;
            }
            if (cb) {
                cb();
            }
        } else {
            bool expected = true;
            if (!keyHeld_.compare_exchange_strong(expected, false)) {
                return;
            }
            std::function<void()> cb;
            {
                std::lock_guard<std::mutex> lk(callbackMutex_);
                cb = onHotkeyReleased;
            }
            if (cb) {
                cb();
            }
        }
        return;
    }

    if (!isKeyDown) {
        std::function<void()> cb;
        {
            std::lock_guard<std::mutex> lk(callbackMutex_);
            cb = onHotkeyPressed;
        }
        if (cb) {
            cb();
        }
    }
}

}  // namespace vocawin
