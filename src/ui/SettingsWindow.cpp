#include "ui/SettingsWindow.h"

#include <sstream>
#include <string>

#if defined(_WIN32)
#include <windows.h>
#include <commctrl.h>
#endif

namespace vocawin {

SettingsWindow::SettingsWindow() = default;

SettingsWindow::~SettingsWindow() {
    hide();
}

bool SettingsWindow::show() {
    if (visible_) {
#if defined(_WIN32)
        if (hwnd_ != nullptr) {
            ShowWindow(static_cast<HWND>(hwnd_), SW_SHOW);
            SetForegroundWindow(static_cast<HWND>(hwnd_));
        }
#endif
        return true;
    }
    if (!load_) {
        return false;
    }
    pending_ = load_();
    if (!createDialog()) {
        return false;
    }
    visible_ = true;
    return true;
}

void SettingsWindow::hide() {
#if defined(_WIN32)
    if (hwnd_ != nullptr) {
        DestroyWindow(static_cast<HWND>(hwnd_));
        hwnd_ = nullptr;
    }
    h_tab_ = h_general_ = h_models_ = h_audio_ = h_hotkeys_ = h_about_ = nullptr;
    h_status_ = h_save_ = h_cancel_ = h_download_ = nullptr;
#endif
    visible_ = false;
}

bool SettingsWindow::pumpMessage() {
#if defined(_WIN32)
    if (hwnd_ == nullptr) return false;
    MSG msg;
    if (PeekMessageW(&msg, static_cast<HWND>(hwnd_), 0, 0, PM_REMOVE)) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
        return true;
    }
    return false;
#else
    return false;
#endif
}

bool SettingsWindow::createDialog() {
#if !defined(_WIN32)
    return false;
#else
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    static const wchar_t kClassName[] = L"VocaWinSettingsClass";
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = reinterpret_cast<WNDPROC>(&SettingsWindow::wndProc);
    wc.hInstance = hInstance;
    wc.lpszClassName = kClassName;
    RegisterClassExW(&wc);  // ignore ERROR_CLASS_ALREADY_EXISTS

    hwnd_ = CreateWindowExW(WS_EX_DLGMODALFRAME, kClassName,
                            L"VocaWin Settings", WS_OVERLAPPED | WS_CAPTION |
                            WS_SYSMENU | WS_MINIMIZEBOX,
                            CW_USEDEFAULT, CW_USEDEFAULT, 560, 480,
                            nullptr, nullptr, hInstance, this);
    if (hwnd_ == nullptr) {
        return false;
    }

    // Tab control
    INITCOMMONCONTROLSEX icc{sizeof(icc), ICC_TAB_CLASSES};
    InitCommonControlsEx(&icc);

    h_tab_ = CreateWindowExW(0, WC_TABCONTROLW, L"",
                             WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                             8, 8, 528, 360,
                             static_cast<HWND>(hwnd_), nullptr, hInstance, nullptr);
    TCITEMW ti{};
    ti.mask = TCIF_TEXT;
    const wchar_t* labels[] = {
        L"General", L"Models", L"Audio", L"Hotkeys", L"About"
    };
    for (int i = 0; i < 5; ++i) {
        ti.pszText = const_cast<wchar_t*>(labels[i]);
        TabCtrl_InsertItem(static_cast<HWND>(h_tab_), i, &ti);
    }

    // Status bar at the bottom
    h_status_ = CreateWindowExW(0, L"STATIC", L"",
                                WS_CHILD | WS_VISIBLE | SS_LEFT,
                                8, 376, 528, 24,
                                static_cast<HWND>(hwnd_), nullptr, hInstance, nullptr);

    h_download_ = CreateWindowExW(0, L"BUTTON", L"Download model",
                                  WS_CHILD | WS_VISIBLE,
                                  8, 410, 140, 28,
                                  static_cast<HWND>(hwnd_),
                                  reinterpret_cast<HMENU>(3), hInstance, nullptr);
    h_save_ = CreateWindowExW(0, L"BUTTON", L"Save",
                              WS_CHILD | WS_VISIBLE | BS_DEFPUSHBUTTON,
                              360, 410, 80, 28,
                              static_cast<HWND>(hwnd_),
                              reinterpret_cast<HMENU>(1), hInstance, nullptr);
    h_cancel_ = CreateWindowExW(0, L"BUTTON", L"Cancel",
                                WS_CHILD | WS_VISIBLE,
                                456, 410, 80, 28,
                                static_cast<HWND>(hwnd_),
                                reinterpret_cast<HMENU>(2), hInstance, nullptr);

    populateGeneralTab();
    populateModelsTab();
    populateAudioTab();
    populateHotkeysTab();
    populateAboutTab();
    ShowWindow(static_cast<HWND>(h_tab_), SW_SHOW);
    showTabPage(0);

    if (is_downloaded_ && is_downloaded_(pending_.model_id)) {
        setStatus(L"Selected model is already on disk. Ready to use.");
    } else {
        setStatus(L"Select a model, then click Download model (~75MB for tiny.en).");
    }

    ShowWindow(static_cast<HWND>(hwnd_), SW_SHOW);
    UpdateWindow(static_cast<HWND>(hwnd_));
    return true;
#endif
}

#if defined(_WIN32)
long long __stdcall SettingsWindow::wndProc(void* hwnd, unsigned int msg,
                                              unsigned long long wparam,
                                              long long lparam) {
    SettingsWindow* self = nullptr;
    if (msg == 0x0081) {  // WM_NCCREATE
        self = reinterpret_cast<SettingsWindow*>(
            reinterpret_cast<CREATESTRUCTW*>(lparam)->lpCreateParams);
        SetWindowLongPtrW(static_cast<HWND>(hwnd), GWLP_USERDATA,
                          reinterpret_cast<LONG_PTR>(self));
    } else {
        self = reinterpret_cast<SettingsWindow*>(
            GetWindowLongPtrW(static_cast<HWND>(hwnd), GWLP_USERDATA));
    }
    if (self == nullptr) {
        return DefWindowProcW(static_cast<HWND>(hwnd), msg, wparam, lparam);
    }

    switch (msg) {
        case WM_COMMAND:
            if (LOWORD(wparam) == 1) {
                if (self->applyChanges()) {
                    self->hide();
                }
            } else if (LOWORD(wparam) == 2) {
                self->hide();
            } else if (LOWORD(wparam) == 3) {
                self->downloadSelectedModel();
            }
            return 0;
        case WM_NOTIFY: {
            const NMHDR* nm = reinterpret_cast<NMHDR*>(lparam);
            if (nm->hwndFrom == self->h_tab_ &&
                nm->code == TCN_SELCHANGE) {
                self->showTabPage(
                    TabCtrl_GetCurSel(static_cast<HWND>(self->h_tab_)));
            }
            return 0;
        }
        case WM_CLOSE:
            self->hide();
            return 0;
        case WM_DESTROY:
            self->visible_ = false;
            self->hwnd_ = nullptr;
            return 0;
    }
    return DefWindowProcW(static_cast<HWND>(hwnd), msg, wparam, lparam);
}

void SettingsWindow::showTabPage(int index) {
    void* pages[] = {h_general_, h_models_, h_audio_, h_hotkeys_, h_about_};
    for (int i = 0; i < 5; ++i) {
        if (pages[i] != nullptr) {
            ShowWindow(static_cast<HWND>(pages[i]),
                       i == index ? SW_SHOW : SW_HIDE);
        }
    }
}
#endif

#if defined(_WIN32)
namespace {

void addLabel(HWND parent, int x, int y, int w, const wchar_t* text) {
    CreateWindowExW(0, L"STATIC", text, WS_CHILD | WS_VISIBLE | SS_LEFT,
                    x, y, w, 20, parent, nullptr, GetModuleHandleW(nullptr), nullptr);
}

void addEdit(HWND parent, int x, int y, int w, int id, const std::wstring& text) {
    CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT", text.c_str(),
                    WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL,
                    x, y, w, 22, parent,
                    reinterpret_cast<HMENU>(static_cast<INT_PTR>(id)),
                    GetModuleHandleW(nullptr), nullptr);
}

void addCheck(HWND parent, int x, int y, int w, int id, const wchar_t* text, bool checked) {
    CreateWindowExW(0, L"BUTTON", text,
                    WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX,
                    x, y, w, 22, parent,
                    reinterpret_cast<HMENU>(static_cast<INT_PTR>(id)),
                    GetModuleHandleW(nullptr), nullptr);
    if (checked) {
        SendMessage(GetDlgItem(parent, id), BM_SETCHECK, BST_CHECKED, 0);
    }
}

void addCombo(HWND parent, int x, int y, int w, int id,
              const wchar_t* const* items, int count, int sel) {
    HWND h = CreateWindowExW(0, L"COMBOBOX", L"",
                             WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST,
                             x, y, w, 200, parent,
                             reinterpret_cast<HMENU>(static_cast<INT_PTR>(id)),
                             GetModuleHandleW(nullptr), nullptr);
    for (int i = 0; i < count; ++i) {
        SendMessageW(h, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(items[i]));
    }
    SendMessageW(h, CB_SETCURSEL, sel, 0);
}

bool isChecked(HWND parent, int id) {
    return SendMessage(GetDlgItem(parent, id), BM_GETCHECK, 0, 0) == BST_CHECKED;
}

int comboSel(HWND parent, int id) {
    return static_cast<int>(SendMessageW(GetDlgItem(parent, id), CB_GETCURSEL, 0, 0));
}

void setText(HWND parent, int id, const std::wstring& text) {
    SetWindowTextW(GetDlgItem(parent, id), text.c_str());
}

std::wstring getText(HWND parent, int id) {
    wchar_t buf[256];
    GetWindowTextW(GetDlgItem(parent, id), buf, 256);
    return std::wstring(buf);
}

}  // namespace
#endif  // _WIN32

void SettingsWindow::populateGeneralTab() {
#if defined(_WIN32)
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    h_general_ = CreateWindowExW(0, L"STATIC", L"", WS_CHILD,
                                 16, 36, 510, 320,
                                 static_cast<HWND>(hwnd_), nullptr, hInstance, nullptr);
    HWND h = static_cast<HWND>(h_general_);

    addLabel(h, 8,  8,  200, L"Launch at startup");
    addCheck(h,  220, 4, 200, 100, L"Run VocaWin when Windows starts",
             pending_.launch_at_startup);

    addLabel(h, 8,  40, 200, L"Sound effects");
    addCheck(h,  220, 36, 200, 101, L"Play start/stop cues",
             pending_.sound_effects);

    addLabel(h, 8,  72, 200, L"Preserve clipboard");
    addCheck(h,  220, 68, 200, 102, L"Save & restore clipboard on paste",
             pending_.preserve_clipboard);

    addLabel(h, 8,  104, 200, L"Show cursor indicator");
    addCheck(h,  220, 100, 200, 103, L"Floating mic icon near cursor",
             pending_.show_cursor_indicator);

    addLabel(h, 8,  136, 200, L"Translate to English");
    addCheck(h,  220, 132, 200, 104, L"Always translate output to English",
             pending_.translate_to_english);

    addLabel(h, 8,  168, 200, L"Language");
    const wchar_t* langs[] = {
        L"auto (detect)", L"en", L"zh", L"de", L"es", L"fr", L"ja", L"ko", L"ru"
    };
    int sel = 0;
    for (int i = 0; i < 9; ++i) {
        std::wstring w = langs[i];
        std::string code(w.begin(), w.end());
        if (code.size() >= 2 && pending_.language == code.substr(0, 2)) {
            sel = i; break;
        }
    }
    addCombo(h, 220, 164, 200, 110, langs, 9, sel);
#endif
}

void SettingsWindow::populateModelsTab() {
#if defined(_WIN32)
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    h_models_ = CreateWindowExW(0, L"STATIC", L"", WS_CHILD,
                                16, 36, 510, 320,
                                static_cast<HWND>(hwnd_), nullptr, hInstance, nullptr);
    HWND h = static_cast<HWND>(h_models_);

    addLabel(h, 8,  8, 200, L"Selected model");
    const wchar_t* models[] = {
        L"tiny.en (English, 39M)", L"base.en (English, 74M)",
        L"small.en (English, 244M)", L"medium.en (English, 769M)",
        L"tiny (Multilingual, 39M)", L"base (Multilingual, 74M)",
        L"small (Multilingual, 244M)", L"medium (Multilingual, 769M)",
    };
    const char* ids[] = {
        "tiny.en", "base.en", "small.en", "medium.en",
        "tiny", "base", "small", "medium"
    };
    int sel = 0;
    for (int i = 0; i < 8; ++i) {
        if (pending_.model_id == ids[i]) { sel = i; break; }
    }
    addCombo(h, 220, 4, 280, 200, models, 8, sel);

    addLabel(h, 8, 40, 200, L"Models directory");
    addEdit(h, 8, 56, 480, 201,
            std::wstring(pending_.models_dir.begin(), pending_.models_dir.end()));

    if (recommend_) {
        addLabel(h, 8, 92, 480, L"(Recommendation based on your hardware is shown in About)");
    }
    addLabel(h, 8, 116, 480,
             L"Click \"Download model\" below to fetch the selected model");
    addLabel(h, 8, 140, 480,
             L"from HuggingFace (HTTPS). tiny.en is ~75MB and good for MVP.");
    addLabel(h, 8, 164, 480,
             L"After download, Save and use Right Ctrl (hold) to record.");
#endif
}

void SettingsWindow::populateAudioTab() {
#if defined(_WIN32)
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    h_audio_ = CreateWindowExW(0, L"STATIC", L"", WS_CHILD,
                               16, 36, 510, 320,
                               static_cast<HWND>(hwnd_), nullptr, hInstance, nullptr);
    HWND h = static_cast<HWND>(h_audio_);

    addLabel(h, 8,  8,  200, L"Silence threshold (RMS, 0-1)");
    addEdit(h, 220, 4,  200, 300, std::to_wstring(pending_.silence_threshold));

    addLabel(h, 8,  40, 200, L"Silence duration (ms)");
    addEdit(h, 220, 36, 200, 301, std::to_wstring(pending_.silence_duration_ms));

    addLabel(h, 8,  72, 200, L"Max recording duration (s)");
    addEdit(h, 220, 68, 200, 302, std::to_wstring(pending_.max_recording_duration_s));

    addLabel(h, 8, 104, 200, L"Text injection method");
    const wchar_t* methods[] = { L"SendInput (Unicode, default)",
                                  L"Clipboard paste (Ctrl+V)" };
    addCombo(h, 220, 100, 200, 310, methods, 2, pending_.text_injection_method);

    addLabel(h, 8, 136, 200, L"Microphone device");
    const wchar_t* devices[] = { L"System default" };
    addCombo(h, 220, 132, 280, 320, devices, 1, 0);
#endif
}

void SettingsWindow::populateHotkeysTab() {
#if defined(_WIN32)
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    h_hotkeys_ = CreateWindowExW(0, L"STATIC", L"", WS_CHILD,
                                 16, 36, 510, 320,
                                 static_cast<HWND>(hwnd_), nullptr, hInstance, nullptr);
    HWND h = static_cast<HWND>(h_hotkeys_);

    addLabel(h, 8,  8, 200, L"Activation mode");
    const wchar_t* modes[] = { L"Push-to-Talk (hold to record)",
                                L"Double-Tap Toggle" };
    addCombo(h, 220, 4, 240, 400, modes, 2, pending_.activation_mode);

    addLabel(h, 8, 40, 200, L"Hotkey (VK code, hex)");
    wchar_t vkbuf[16];
    swprintf(vkbuf, 16, L"0x%02X", pending_.hotkey_vk_code);
    addEdit(h, 220, 36, 80, 401, vkbuf);

    addLabel(h, 8, 72, 200, L"Double-tap threshold (ms)");
    addEdit(h, 220, 68, 80, 402, std::to_wstring(
        static_cast<int>(pending_.double_tap_threshold_ms)));

    addLabel(h, 8, 104, 480,
             L"Common VK codes: 0xA2 (Left Ctrl), 0xA3 (Right Ctrl),");
    addLabel(h, 8, 124, 480, L"0x12 (Alt), 0x20 (Space), 0x11 (Ctrl)");
#endif
}

void SettingsWindow::populateAboutTab() {
#if defined(_WIN32)
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    h_about_ = CreateWindowExW(0, L"STATIC", L"", WS_CHILD,
                               16, 36, 510, 320,
                               static_cast<HWND>(hwnd_), nullptr, hInstance, nullptr);
    HWND h = static_cast<HWND>(h_about_);

    addLabel(h, 8, 8, 480, L"VocaWin 0.1.0 (MVP)");
    addLabel(h, 8, 32, 480,
             L"100% offline voice-to-text for Windows.");
    addLabel(h, 8, 56, 480, L"https://vocawin.com");
    addLabel(h, 8, 80, 480, L"License: AGPL-3.0");
    addLabel(h, 8, 120, 480, L"System:");
    if (system_info_) {
        addLabel(h, 8, 140, 480, system_info_().c_str());
    }
    addLabel(h, 8, 180, 480, L"Recommendation:");
    if (recommend_) {
        // Placeholder - in real use we'd query SystemInfo here.
        addLabel(h, 8, 200, 480, L"See ModelManager::recommendModel()");
    }
    if (about_text_) {
        addLabel(h, 8, 240, 480, about_text_().c_str());
    }
#endif
}

namespace {

bool convertWStringToUtf8(const std::wstring& wide, std::string& out) {
    if (wide.empty()) {
        out.clear();
        return true;
    }
#if defined(_WIN32)
    const int size = ::WideCharToMultiByte(
        CP_UTF8, 0, wide.data(), static_cast<int>(wide.size()),
        nullptr, 0, nullptr, nullptr);
    if (size <= 0) {
        out.clear();
        return false;
    }
    out.resize(static_cast<std::size_t>(size));
    ::WideCharToMultiByte(
        CP_UTF8, 0, wide.data(), static_cast<int>(wide.size()),
        out.data(), size, nullptr, nullptr);
    return true;
#else
    // Fallback for non-Windows: assume narrow representation.
    out.assign(wide.begin(), wide.end());
    return true;
#endif
}

}  // namespace

void SettingsWindow::readControlsInto(Settings& out) const {
#if defined(_WIN32)
    HWND h = static_cast<HWND>(h_general_);
    if (h == nullptr) return;
    out.launch_at_startup = isChecked(h, 100);
    out.sound_effects = isChecked(h, 101);
    out.preserve_clipboard = isChecked(h, 102);
    out.show_cursor_indicator = isChecked(h, 103);
    out.translate_to_english = isChecked(h, 104);
    int langSel = comboSel(h, 110);
    const char* langMap[] = {"auto", "en", "zh", "de", "es", "fr", "ja", "ko", "ru"};
    if (langSel >= 0 && langSel < 9) out.language = langMap[langSel];

    h = static_cast<HWND>(h_models_);
    if (h != nullptr) {
        int msel = comboSel(h, 200);
        const char* modelMap[] = {
            "tiny.en", "base.en", "small.en", "medium.en",
            "tiny", "base", "small", "medium"
        };
        if (msel >= 0 && msel < 8) out.model_id = modelMap[msel];
        const std::wstring md = getText(h, 201);
        out.models_dir = std::string(md.begin(), md.end());
    }

    h = static_cast<HWND>(h_audio_);
    if (h != nullptr) {
        try {
            out.silence_threshold = std::stof(getText(h, 300));
            out.silence_duration_ms = static_cast<std::uint32_t>(
                std::stoul(getText(h, 301)));
            out.max_recording_duration_s = static_cast<std::uint32_t>(
                std::stoul(getText(h, 302)));
        } catch (...) { /* keep defaults on parse error */ }
        out.text_injection_method = comboSel(h, 310);
    }

    h = static_cast<HWND>(h_hotkeys_);
    if (h != nullptr) {
        out.activation_mode = comboSel(h, 400);
        const std::wstring vk = getText(h, 401);
        try {
            out.hotkey_vk_code = static_cast<std::uint32_t>(
                std::stoul(vk, nullptr, 16));
        } catch (...) { /* keep */ }
        try {
            out.double_tap_threshold_ms = static_cast<double>(
                std::stoi(getText(h, 402)));
        } catch (...) { /* keep */ }
    }
#endif
}

bool SettingsWindow::applyChanges() {
    Settings s = pending_;
    readControlsInto(s);
    if (save_) {
        const bool ok = save_(s);
        if (ok) pending_ = s;
        return ok;
    }
    return false;
}

void SettingsWindow::setStatus(const std::wstring& text) {
#if defined(_WIN32)
    if (h_status_ != nullptr) {
        SetWindowTextW(static_cast<HWND>(h_status_), text.c_str());
    }
#else
    (void)text;
#endif
}

bool SettingsWindow::downloadSelectedModel() {
    if (!download_) {
        setStatus(L"Download is not available.");
        return false;
    }
    Settings s = pending_;
    readControlsInto(s);
    pending_ = s;

    // Persist selection first so AppController uses the same model id.
    if (save_) {
        (void)save_(s);
    }

    setStatus(L"Downloading model (this may take a few minutes)...");
#if defined(_WIN32)
    if (h_download_ != nullptr) {
        EnableWindow(static_cast<HWND>(h_download_), FALSE);
    }
    // Keep the dialog responsive during a long blocking download.
    MSG msg;
    auto pump = [&]() {
        while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    };
    pump();
#endif

    float lastShown = -1.0f;
    const bool ok = download_(s.model_id, [&](float p) {
        if (p - lastShown < 0.05f && p < 0.99f) {
            return;
        }
        lastShown = p;
        const int pct = static_cast<int>(p * 100.0f + 0.5f);
        setStatus(L"Downloading model... " + std::to_wstring(pct) + L"%");
#if defined(_WIN32)
        pump();
#endif
    });

#if defined(_WIN32)
    if (h_download_ != nullptr) {
        EnableWindow(static_cast<HWND>(h_download_), TRUE);
    }
#endif

    if (ok) {
        setStatus(L"Model downloaded and loaded. Hold Right Ctrl to record.");
    } else {
        setStatus(L"Download failed. Check network / logs and try again.");
    }
    return ok;
}

}  // namespace vocawin
