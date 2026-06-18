#include "input/ClipboardManager.h"

#include <cstring>

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

ClipboardManager::ClipboardManager() {
#if defined(_WIN32)
    // Create a tiny message-only window so we have a valid clipboard owner
    // for restore(). This lets the system keep the saved data alive.
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    static const wchar_t kClassName[] = L"VocaWinClipboardOwner";
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = DefWindowProcW;
    wc.hInstance = hInstance;
    wc.lpszClassName = kClassName;
    RegisterClassExW(&wc);
    hwnd_ = CreateWindowExW(0, kClassName, L"", 0, 0, 0, 0, 0,
                            HWND_MESSAGE, nullptr, hInstance, nullptr);
#endif
}

ClipboardManager::~ClipboardManager() {
    clearSaved();
#if defined(_WIN32)
    if (hwnd_ != nullptr) {
        DestroyWindow(static_cast<HWND>(hwnd_));
        hwnd_ = nullptr;
    }
#endif
}

bool ClipboardManager::save() {
    clearSaved();
#if defined(_WIN32)
    if (!OpenClipboard(nullptr)) {
        return false;
    }
    bool captured = false;
    UINT format = 0;
    while ((format = EnumClipboardFormats(format)) != 0) {
        HANDLE hData = GetClipboardData(format);
        if (hData == nullptr) {
            continue;
        }
        SIZE_T size = GlobalSize(hData);
        if (size == 0) {
            continue;
        }
        void* src = GlobalLock(hData);
        if (src == nullptr) {
            continue;
        }
        ClipboardEntry entry;
        entry.format = static_cast<std::uint32_t>(format);
        entry.data.assign(static_cast<std::uint8_t*>(src),
                          static_cast<std::uint8_t*>(src) + size);
        GlobalUnlock(hData);
        saved_.push_back(std::move(entry));
        captured = true;
    }
    CloseClipboard();
    hasSaved_ = captured;
    return captured;
#else
    hasSaved_ = false;
    return false;
#endif
}

void ClipboardManager::restore() {
#if defined(_WIN32)
    if (!hasSaved_ || saved_.empty()) {
        return;
    }
    if (!OpenClipboard(static_cast<HWND>(hwnd_))) {
        return;
    }
    EmptyClipboard();
    for (const auto& entry : saved_) {
        HGLOBAL hMem = GlobalAlloc(GMEM_MOVEABLE, entry.data.size());
        if (hMem == nullptr) {
            continue;
        }
        void* dst = GlobalLock(hMem);
        if (dst == nullptr) {
            GlobalFree(hMem);
            continue;
        }
        std::memcpy(dst, entry.data.data(), entry.data.size());
        GlobalUnlock(hMem);
        SetClipboardData(static_cast<UINT>(entry.format), hMem);
    }
    CloseClipboard();
    hasSaved_ = false;
    clearSaved();
#endif
}

bool ClipboardManager::setText(const std::wstring& text) {
#if defined(_WIN32)
    if (!OpenClipboard(nullptr)) {
        return false;
    }
    EmptyClipboard();
    const SIZE_T bytes = (text.size() + 1) * sizeof(wchar_t);
    HGLOBAL hMem = GlobalAlloc(GMEM_MOVEABLE, bytes);
    if (hMem == nullptr) {
        CloseClipboard();
        return false;
    }
    void* dst = GlobalLock(hMem);
    if (dst == nullptr) {
        GlobalFree(hMem);
        CloseClipboard();
        return false;
    }
    std::memcpy(dst, text.c_str(), bytes);
    GlobalUnlock(hMem);
    SetClipboardData(CF_UNICODETEXT, hMem);
    CloseClipboard();
    return true;
#else
    (void)text;
    return false;
#endif
}

bool ClipboardManager::hasSavedData() const {
    return hasSaved_;
}

void ClipboardManager::clearSaved() {
    saved_.clear();
    hasSaved_ = false;
}

}  // namespace vocawin
