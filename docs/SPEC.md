# VocaWin — Engineering & Product Specification

## 1. Product Overview

**VocaWin** is a 100% offline, privacy-first voice-to-text application for Windows 10/11. It captures audio from the microphone, transcribes it locally using Whisper models, and injects the resulting text at the cursor position in any application. No cloud, no accounts, no telemetry.

### 1.1 Product Positioning

- Part of the **Voca ecosystem** (VocaMac, VocaLinux, VocaWin)
- **Target users**: Developers, power users, privacy-conscious individuals, accessibility users
- **Competitive differentiation**: 100% offline (vs. Windows Voice Typing which sends some data to Microsoft), open source, GPU-accelerated, free

### 1.2 Core Principles (non-negotiable)

| Principle | Requirement |
|---|---|
| 100% Offline | Zero network calls during normal operation |
| Privacy First | No audio storage, no telemetry, no analytics |
| Open Source | Public repo, auditable code |
| Free Forever | No premium tiers, no subscriptions |
| Windows Native | Feels like a first-party Windows app |

---

## 2. Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| **Language** | C++17/20 | Direct whisper.cpp integration, zero-cost abstractions, no runtime dependency |
| **UI Framework** | WinUI 3 (Windows App SDK) | Modern Windows UI, XAML Islands, system tray support, forward-looking |
| **Speech Engine** | whisper.cpp (MIT) | Best C++ Whisper implementation, native GPU backends, zero-allocation runtime |
| **GPU - CUDA** | `-DGGML_CUDA=1` | NVIDIA GPU acceleration (best perf) |
| **GPU - Vulkan** | `-DGGML_VULKAN=1` | Cross-vendor GPU (NVIDIA, AMD, Intel) |
| **GPU - DirectML** | ONNX Runtime + DirectML EP | Any DirectX 12 GPU (future path, see §2.1) |
| **Audio Capture** | WASAPI (Windows Audio Session API) | Native Windows audio, lowest latency |
| **Text Injection** | `SendInput` (Unicode) + Clipboard paste | System-wide text delivery |
| **Global Hotkeys** | `SetWindowsHookExW(WH_KEYBOARD_LL)` | Low-level keyboard hook for push-to-talk |
| **Build System** | CMake 3.21+ | whisper.cpp uses CMake, industry standard for C++ |
| **Dependency Manager** | vcpkg | Microsoft's C++ package manager, integrates with CMake |
| **Installer** | WiX Toolset v4 | MSI packages, standard Windows installer |
| **Auto-Update** | GitHub Releases API + custom updater | Check for updates, download, prompt restart |
| **Logging** | Custom file logger + Windows Event Log | Debug and diagnostics |
| **Testing** | Google Test (gtest) | C++ unit testing framework |

### 2.1 DirectML Strategy

whisper.cpp does **not** have a native DirectML backend. The plan is:

**Phase 1 (Launch):** CUDA + Vulkan backends via whisper.cpp. Vulkan covers all GPU vendors.
**Phase 2 (Post-launch):** Add DirectML via ONNX Runtime with DirectML execution provider as an alternative inference path. This requires:
- Converting Whisper models to ONNX format (tools exist)
- Integrating ONNX Runtime as a separate inference backend
- Offering it as a fallback when neither CUDA nor Vulkan drivers are available but DirectX 12 is

This is viable because DirectML works on **any** DirectX 12 GPU (including older hardware that may not support Vulkan well).

---

## 3. Architecture

### 3.1 High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        VocaWin Application                       │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────────┐ │
│  │   WinUI 3   │  │   System    │  │     Windows App SDK      │ │
│  │ Settings UI │  │    Tray     │  │    (Windowing, etc.)     │ │
│  │ Onboarding  │  │  NotifyIcon │  │                          │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────────────────────┘ │
│         │                │                                       │
│  ┌──────┴────────────────┴──────────────────────────────────┐   │
│  │                    AppController                          │   │
│  │          (state machine, orchestration, callbacks)        │   │
│  └──┬──────┬──────────┬──────────────┬───────────────┬──────┘   │
│     │      │          │              │               │          │
│  ┌──┴──┐┌──┴───┐┌─────┴─────┐┌──────┴──────┐┌───────┴──────┐  │
│  │Hotkey││Audio ││   Model   ││   Speech    ││    Text      │  │
│  │ Mgr  ││Capture││  Manager  ││   Engine    ││  Injector    │  │
│  │      ││      ││           ││             ││              │  │
│  │SetWin││WASAPI││Download/  ││whisper.cpp  ││SendInput +   │  │
│  │HookEx││      ││Load/Select││CUDA/Vulkan  ││Clipboard     │  │
│  └──────┘└──────┘└───────────┘└─────────────┘└──────────────┘  │
│                                                                  │
│  ┌──────────┐┌──────────┐┌──────────┐                           │
│  │ Settings ││  Sound   ││  Logger  │                           │
│  │  Store   ││Feedback  ││          │                           │
│  │(JSON)    ││(WAV)     ││(File)    │                           │
│  └──────────┘└──────────┘└──────────┘                           │
└──────────────────────────────────────────────────────────────────┘
```

### 3.2 Threading Model

```
Thread 1 (Main/UI)         Thread 2 (Audio)           Thread 3 (Inference)
┌──────────────────┐       ┌──────────────────┐       ┌──────────────────┐
│ WinUI 3 event    │       │ WASAPI capture   │       │ whisper.cpp      │
│ loop             │       │ callback         │       │ full() / stream  │
│                  │       │                  │       │                  │
│ - Tray icon      │       │ - Read samples   │       │ - Dequeue audio │
│ - Settings UI    │◄──────│ - RMS calc       │──────►│ - Transcribe    │
│ - State updates  │       │ - Buffer into    │       │ - Filter output │
│ - Notifications  │       │   ring buffer    │       │ - Callback with │
│                  │       │ - Detect silence │       │   text result   │
└──────────────────┘       └──────────────────┘       └──────────────────┘
        ▲                                                        │
        │                                                        │
        └────────── text result callback ────────────────────────┘
                    (back to UI thread via dispatcher)
```

**Key threading rules:**
- Audio callback runs on a WASAPI high-priority thread — **must not block**
- Whisper inference runs on a dedicated thread — can be slow (1-5s)
- All UI updates marshaled to the main thread via WinUI dispatcher
- Shared state protected by `std::mutex` with fine-grained locking
- Lock-free ring buffer between audio capture and inference threads

### 3.3 State Machine

```
                    ┌──────────────┐
                    │  NOT_LOADED  │ (App starts, no model)
                    └──────┬───────┘
                           │ loadModel()
                           ▼
                    ┌──────────────┐
           ┌───────│     IDLE     │◄──────────────────────┐
           │       └──────┬───────┘                       │
           │              │ hotkey pressed                 │
           │              ▼                               │
           │       ┌──────────────┐                       │
           │       │  RECORDING   │                       │
           │       └──────┬───────┘                       │
           │              │ hotkey released / silence /   │
           │              │ max duration                  │
           │              ▼                               │
           │       ┌──────────────┐  success              │
           │       │  PROCESSING  │───────────────────────┘
           │       └──────┬───────┘
           │              │ error
           │              ▼
           │       ┌──────────────┐   3s timeout
           └───────│    ERROR     │───────────┘
                   └──────────────┘
```

---

## 4. Module Design

### 4.1 Project Structure

```
vocawin/
├── src/
│   ├── main.cpp                          # Entry point, WinUI app init
│   ├── app/
│   │   ├── App.xaml.h/.cpp               # WinUI Application class
│   │   ├── AppController.h/.cpp          # Central orchestrator
│   │   └── SingleInstance.h/.cpp         # Named mutex enforcement
│   ├── ui/
│   │   ├── TrayIcon.h/.cpp               # System tray via Windows App SDK
│   │   ├── TrayIconManager.h/.cpp        # Icon state transitions
│   │   ├── SettingsWindow.xaml.h/.cpp    # Settings dialog (WinUI 3 XAML)
│   │   ├── OnboardingWindow.xaml.h/.cpp  # First-run wizard
│   │   ├── AboutDialog.xaml.h/.cpp       # About page
│   │   └── OverlayWindow.h/.cpp          # Floating mic indicator near cursor
│   ├── audio/
│   │   ├── AudioCapture.h/.cpp           # WASAPI microphone capture
│   │   ├── AudioBuffer.h/.cpp            # Lock-free ring buffer (SPSC)
│   │   ├── SilenceDetector.h/.cpp        # RMS-based VAD
│   │   └── SoundFeedback.h/.cpp          # WAV playback for start/stop sounds
│   ├── speech/
│   │   ├── WhisperEngine.h/.cpp          # whisper.cpp integration
│   │   ├── ModelManager.h/.cpp           # Download, store, load, switch models
│   │   └── ModelInfo.h/.cpp              # Model metadata, GPU detection, recommendations
│   ├── input/
│   │   ├── HotkeyManager.h/.cpp          # Global keyboard hook
│   │   ├── TextInjector.h/.cpp           # SendInput + clipboard text injection
│   │   └── ClipboardManager.h/.cpp       # Clipboard save/restore
│   ├── platform/
│   │   ├── Autostart.h/.cpp              # Registry Run key
│   │   ├── SystemInfo.h/.cpp             # CPU, RAM, GPU detection
│   │   ├── GpuDetector.h/.cpp            # CUDA/Vulkan capability detection
│   │   └── Notification.h/.cpp           # Windows Toast notifications
│   ├── config/
│   │   ├── Settings.h/.cpp               # Settings model (serializable to JSON)
│   │   └── SettingsStore.h/.cpp          # JSON file read/write at %APPDATA%
│   └── util/
│       ├── Logger.h/.cpp                 # File logger with rotation
│       └── Util.h                        # Common helpers
├── whisper.cpp/                          # Git submodule (upstream whisper.cpp)
├── resources/
│   ├── icons/                            # App icon, tray icons (ICO, PNG)
│   │   ├── vocawin.ico
│   │   ├── tray-idle.ico
│   │   ├── tray-recording.ico
│   │   ├── tray-processing.ico
│   │   └── tray-error.ico
│   └── sounds/                           # Audio feedback WAV files
│       ├── start.wav
│       ├── stop.wav
│       └── error.wav
├── installer/
│   └── VocaWin.wxs                       # WiX v4 MSI definition
├── tests/
│   ├── CMakeLists.txt
│   ├── test_audio_buffer.cpp
│   ├── test_silence_detector.cpp
│   ├── test_model_info.cpp
│   ├── test_settings.cpp
│   └── test_text_injector.cpp
├── scripts/
│   ├── generate_sounds.py                # Generate WAV feedback sounds
│   └── download_models.py                # Download GGML models for testing
├── web/                                  # (existing) Landing page
├── .github/
│   └── workflows/
│       ├── ci.yml                        # Build + test on Windows
│       ├── release.yml                   # Build MSI, create GitHub Release
│       └── deploy-pages.yml              # (existing) Website deployment
├── CMakeLists.txt                        # Root CMake config
├── CMakePresets.json                     # Build presets (Debug/Release, GPU variants)
├── vcpkg.json                            # Dependency manifest
├── .gitignore
├── LICENSE                               # AGPL-3.0
├── README.md
├── CONTRIBUTING.md
└── AGENTS.md                             # AI coding guidelines
```

### 4.2 Module Specifications

#### 4.2.1 `AppController` — Central Orchestrator

The brain of the application. Owns all service instances and coordinates the recording flow.

```cpp
class AppController {
public:
    enum class State { NotLoaded, Idle, Recording, Processing, Error };

    void initialize();     // Load settings, detect hardware, init services
    void shutdown();       // Clean teardown

    // Recording lifecycle
    void startRecording();
    void stopRecordingAndTranscribe();
    void cancelRecording();

    // State
    State state() const;
    std::wstring lastError() const;

    // Callbacks (registered by UI layer)
    std::function<void(State)> onStateChanged;
    std::function<void(float)> onAudioLevelChanged;  // 0.0-1.0
    std::function<void(std::wstring)> onTranscriptionComplete;

private:
    State m_state{State::NotLoaded};
    AudioCapture m_audioCapture;
    WhisperEngine m_speechEngine;
    ModelManager m_modelManager;
    HotkeyManager m_hotkeyManager;
    TextInjector m_textInjector;
    ClipboardManager m_clipboardManager;
    SoundFeedback m_soundFeedback;
    Settings m_settings;
    Logger m_logger;
    std::mutex m_stateMutex;
    std::thread m_inferenceThread;
};
```

#### 4.2.2 `AudioCapture` — WASAPI Microphone Capture

Direct WASAPI integration (no SDL2 dependency needed):

```cpp
class AudioCapture {
public:
    struct Config {
        uint32_t sampleRate = 16000;   // Whisper requirement
        uint32_t channels = 1;         // Mono
        uint32_t bufferDurationMs = 100;
        int deviceIndex = -1;          // -1 = system default
    };

    void start(Config config);
    void stop();
    std::vector<float> getBuffer();    // Returns collected Float32 samples
    void clearBuffer();

    // Callbacks
    std::function<void(const float*, size_t)> onAudioData;     // Raw samples
    std::function<void(float)> onAudioLevel;                   // RMS 0-1
    std::function<void()> onSilenceDetected;                   // Silence timeout

    // Device enumeration
    static std::vector<AudioDevice> enumerateDevices();

private:
    void wasapiCaptureThread();
    // WASAPI handles: IMMDevice, IAudioClient, IAudioCaptureClient
};
```

**Audio pipeline:**
```
Microphone → IMMDevice → IAudioClient (16kHz, mono, Float32)
                       → IAudioCaptureClient → buffer callback
                       → resample if needed
                       → accumulate into AudioBuffer
                       → compute RMS for level meter
                       → detect silence via SilenceDetector
```

**Device format negotiation:** Query device's supported formats. If native 16kHz mono isn't supported, use the closest supported format (typically 44.1/48kHz stereo) and resample.

#### 4.2.3 `WhisperEngine` — whisper.cpp Integration

```cpp
class WhisperEngine {
public:
    struct GpuBackend {
        enum Type { None, Cuda, Vulkan, DirectML };
        Type type;
        std::string name;         // "NVIDIA RTX 4080", "AMD Radeon RX 7900", etc.
        size_t vramBytes;
    };

    void loadModel(const std::filesystem::path& modelPath,
                   const GpuBackend& gpu,
                   int nThreads);
    void unloadModel();
    void setLanguage(const std::string& lang);  // "auto", "en", "es", etc.
    void setTranslateMode(bool translate);       // Translate to English

    struct Result {
        std::wstring text;
        std::string language;
        float confidence;
    };

    std::optional<Result> transcribe(const std::vector<float>& audioData);

private:
    whisper_context* m_ctx{nullptr};
    whisper_full_params m_params;
    std::string m_language{"auto"};
    bool m_translate{false};

    // Hallucination filtering
    static std::wstring filterText(const std::string& raw);
    // Removes: [BLANK_AUDIO], [NO_SPEECH], [Music], <minimal>, etc.
};
```

**GPU Backend Selection Logic:**
```
1. Query CUDA via nvidia-smi or CUDA API → if available, prefer CUDA (fastest)
2. Query Vulkan via vulkaninfo → if available, use Vulkan
3. Fall back to CPU with AVX2/AVX512 auto-detection
4. (Phase 2) Query DirectML via DXGI → if available and no CUDA/Vulkan
```

**Multi-GPU build strategy:** Build whisper.cpp with all GPU backends compiled in. At runtime, detect available hardware and select the best backend. Ship a single binary that adapts.

#### 4.2.4 `ModelManager` — Model Download & Storage

```cpp
class ModelManager {
public:
    struct ModelInfo {
        std::string id;            // "tiny", "base", "small", "medium", "large-v3"
        std::string displayName;   // "Tiny (39M params)"
        std::string url;           // HuggingFace download URL
        size_t fileSizeBytes;
        size_t ramRequiredBytes;
        bool isDownloaded;
        bool isActive;
    };

    static std::vector<ModelInfo> getAvailableModels();
    std::vector<ModelInfo> getLocalModels() const;

    // Download with progress callback
    void downloadModel(const std::string& modelId,
                       std::function<void(float progress)> onProgress);
    void deleteModel(const std::string& modelId);

    std::filesystem::path getModelPath(const std::string& modelId) const;

    // Hardware-based recommendation
    static ModelInfo recommendModel();

private:
    std::filesystem::path m_modelsDir; // %LOCALAPPDATA%/VocaWin/models/
};
```

**Model catalog (HuggingFace URLs):**

| ID | Size | RAM Needed | Recommended For |
|---|---|---|---|
| tiny | 75 MB | ~273 MB | Quick notes, low-end hardware |
| base | 142 MB | ~388 MB | 4-8 GB RAM, no GPU |
| small | 466 MB | ~852 MB | 8-16 GB RAM with GPU |
| medium | 1.5 GB | ~2.1 GB | 16+ GB RAM + GPU |
| large-v3 | 2.9 GB | ~3.9 GB | 24+ GB RAM + powerful GPU |
| large-v3-turbo | 809 MB | ~1.2 GB | Best accuracy/speed ratio |

**Recommendation algorithm (adapted from VocaMac):**
```
GPU Detected (CUDA or Vulkan):
  VRAM >= 8 GB  → medium
  VRAM >= 4 GB  → small
  VRAM >= 2 GB  → base
  No VRAM info  → base

CPU Only:
  RAM >= 32 GB  → small
  RAM >= 16 GB  → base
  RAM >= 8 GB   → tiny
  RAM < 8 GB    → tiny (with warning)
```

#### 4.2.5 `HotkeyManager` — Global Keyboard Hook

```cpp
class HotkeyManager {
public:
    enum class ActivationMode { PushToTalk, DoubleTapToggle };

    struct Config {
        uint32_t virtualKeyCode = VK_RCONTROL;  // Default: Right Ctrl
        ActivationMode mode = ActivationMode::PushToTalk;
        double doubleTapThresholdMs = 400;
    };

    void start(Config config);
    void stop();

    // Callbacks
    std::function<void()> onHotkeyPressed;
    std::function<void()> onHotkeyReleased;

private:
    static LRESULT CALLBACK lowLevelKeyboardProc(int nCode, WPARAM wParam, LPARAM lParam);
    HHOOK m_hook{nullptr};
    Config m_config;

    // Safety: force-release after timeout
    std::thread m_safetyTimer;
    void startSafetyTimer(std::chrono::seconds timeout);
};
```

**Default hotkey:** Right Ctrl.

**Push-to-Talk flow:**
1. Key down → fire `onHotkeyPressed` → start recording
2. Key up → fire `onHotkeyReleased` → stop and transcribe

**Double-tap toggle flow:**
1. First key down → start timer (400ms)
2. Second key down within threshold → toggle recording on
3. Next double-tap → toggle recording off

**Safety mechanisms (from VocaMac):**
- Safety timer: force stop if key-up event is lost (e.g., focus change)
- Recovery: if key-down fires while already recording (missed key-up), treat as stop

#### 4.2.6 `TextInjector` — System-Wide Text Delivery

```cpp
class TextInjector {
public:
    struct Config {
        bool preserveClipboard = true;
        uint32_t pasteDelayMs = 100;          // Wait for clipboard to settle
        uint32_t restoreDelayMs = 2000;       // Wait before restoring clipboard
    };

    void injectText(const std::wstring& text);

private:
    ClipboardManager m_clipboard;

    // Method 1: SendInput with Unicode (primary)
    void injectViaSendInput(const std::wstring& text);

    // Method 2: Clipboard paste (fallback for problematic apps)
    void injectViaClipboardPaste(const std::wstring& text);

    // SendInput Unicode injection (works regardless of keyboard layout)
    void sendUnicodeText(const std::wstring& text);
};
```

**Primary injection method — `SendInput` with `KEYEVENTF_UNICODE`:**
```cpp
void TextInjector::sendUnicodeText(const std::wstring& text) {
    std::vector<INPUT> inputs;
    inputs.reserve(text.size() * 2);

    for (wchar_t ch : text) {
        INPUT down = {};
        down.type = INPUT_KEYBOARD;
        down.ki.wScan = ch;
        down.ki.dwFlags = KEYEVENTF_UNICODE;
        inputs.push_back(down);

        INPUT up = down;
        up.ki.dwFlags |= KEYEVENTF_KEYUP;
        inputs.push_back(up);
    }

    SendInput(inputs.size(), inputs.data(), sizeof(INPUT));
}
```

**Fallback method — Clipboard paste (adapted from VocaMac):**
1. Save current clipboard (all formats, deep copy)
2. Set clipboard text to transcribed text
3. Sleep 100ms
4. Simulate Ctrl+V via SendInput
5. Sleep 2s
6. Restore original clipboard

**When to use which method:**
- Default: SendInput Unicode (works everywhere, preserves clipboard)
- User can opt into clipboard-paste mode in settings (for apps that don't receive SendInput well)

#### 4.2.7 `ClipboardManager` — Clipboard Preservation

```cpp
class ClipboardManager {
public:
    void save();                          // Deep-copy all clipboard formats
    void restore();                       // Restore saved clipboard
    void setText(const std::wstring& text); // Set clipboard to text

private:
    struct ClipboardData {
        UINT format;
        std::vector<uint8_t> data;
    };
    std::vector<ClipboardData> m_savedData;
};
```

**Deep copy approach:** Enumerate all clipboard formats (CF_TEXT, CF_UNICODETEXT, CF_HDROP, CF_DIB, etc.), read raw bytes for each, store. On restore, write all formats back.

#### 4.2.8 `SilenceDetector` — Voice Activity Detection

```cpp
class SilenceDetector {
public:
    struct Config {
        float threshold = 0.01f;     // RMS threshold
        uint32_t durationMs = 2000;  // Silence duration before triggering
    };

    void feedSample(float sample);
    void feedBuffer(const float* data, size_t len);
    void reset();

    bool isSilent() const;
    std::chrono::milliseconds silenceDuration() const;

    std::function<void()> onSilenceTimeout;

private:
    float m_threshold;
    uint32_t m_silenceDurationMs;
    std::chrono::steady_clock::time_point m_lastLoudTime;
};
```

---

## 5. UI Design

### 5.1 System Tray

**Tray icon states:**

| State | Icon Color | Tooltip |
|---|---|---|
| Idle | Blue (#0078D4) | "VocaWin — Ready" |
| Recording | Red (#E81123) | "VocaWin — Recording..." |
| Processing | Purple (#886CE4) | "VocaWin — Processing..." |
| Error | Yellow/Orange (#FFB900) | "VocaWin — Error: {message}" |
| No Model | Gray | "VocaWin — No model loaded" |

**Right-click context menu:**
```
┌─────────────────────────┐
│ ► Start Recording       │  (or "Stop Recording" when active)
├─────────────────────────┤
│   Settings...           │
│   Open Log Folder       │
│   Check for Updates     │
├─────────────────────────┤
│   About VocaWin         │
│   Quit                  │
└─────────────────────────┘
```

**Left-click:** Toggle recording (alternative to hotkey)

### 5.2 Settings Window (WinUI 3)

Tabbed interface with 5 tabs:

**Tab 1 — General:**
- Launch at startup (toggle)
- Sound effects (toggle)
- Show cursor indicator (toggle)
- Text injection method: SendInput / Clipboard paste
- Preserve clipboard (toggle)
- Log level dropdown (debug/info/warn/error)
- Language: Auto-detect / specific language dropdown

**Tab 2 — Models:**
- Current model display (name, size, status)
- Model grid/list with download buttons:
  - tiny (75 MB) — Quick notes
  - base (142 MB) — Everyday use
  - small (466 MB) — Better accuracy
  - medium (1.5 GB) — High accuracy
  - large-v3 (2.9 GB) — Maximum accuracy
  - large-v3-turbo (809 MB) — Best ratio
- Download progress bar (active downloads)
- Delete button for downloaded models
- Hardware recommendation banner ("Based on your hardware, we recommend: base")

**Tab 3 — Audio:**
- Microphone device selector (dropdown)
- Audio level meter (live VU meter)
- Silence threshold slider
- Silence duration slider (1-10 seconds)
- Max recording duration selector (15s, 30s, 60s, 120s, 300s)

**Tab 4 — Hotkeys:**
- Activation mode: Push-to-talk / Double-tap toggle
- Hotkey selector (click to record new key)
- Double-tap threshold slider (200-800ms)

**Tab 5 — About:**
- App name + version
- VocaWin logo
- System info (CPU, RAM, GPU, Windows version)
- Links: GitHub, VocaMac, VocaLinux
- License info
- "Check for Updates" button

### 5.3 Onboarding Wizard (First Run)

4-step wizard shown on first launch:

1. **Welcome** — "Welcome to VocaWin. Let's get you set up."
2. **Microphone** — Select and test microphone. Live VU meter. "Say something to test..."
3. **Model** — Hardware detection result shown. Recommended model pre-selected. Download starts. Progress bar.
4. **Hotkey** — Default hotkey shown (Right Ctrl). Option to change. Brief push-to-talk vs toggle explanation. "You're all set!"

### 5.4 Floating Cursor Indicator (Optional)

A small floating window near the text cursor:
- Uses Windows UI Automation API (`IUIAutomation`) to detect caret position
- Shows a small microphone icon during recording (red), processing (purple)
- Transparent to mouse clicks (`WS_EX_TRANSPARENT`)
- Always on top (`WS_EX_TOPMOST`)
- Follows cursor position (poll every 500ms)
- Can be disabled in settings

---

## 6. Complete Feature Set

### 6.1 Core Features (Must-Have for v0.1.0)

| # | Feature | Description |
|---|---|---|
| F1 | System-wide text injection | Transcribed text appears at cursor in any app |
| F2 | Push-to-talk | Hold hotkey to record, release to transcribe |
| F3 | Double-tap toggle | Double-tap hotkey to start/stop recording |
| F4 | System tray presence | Colored icon indicating state, context menu |
| F5 | Settings UI | WinUI 3 settings window with 5 tabs |
| F6 | Onboarding wizard | First-run setup (mic, model, hotkey) |
| F7 | Model management | Download, switch, delete Whisper models |
| F8 | Smart model selection | Auto-detect hardware, recommend optimal model |
| F9 | GPU acceleration | CUDA (NVIDIA), Vulkan (all GPUs) |
| F10 | CPU fallback | AVX2/AVX512 optimized CPU inference |
| F11 | 99+ languages | Auto-detect or specify language |
| F12 | Silence detection | Auto-stop recording after configurable silence |
| F13 | Sound effects | Start/stop recording audio feedback |
| F14 | Configurable hotkey | Change the activation key |
| F15 | Launch at startup | Windows Registry Run key |
| F16 | Single instance | Named mutex prevents multiple instances |
| F17 | Audio device selection | Choose microphone from available devices |
| F18 | Logging | File logging with rotation for debugging |

### 6.2 Nice-to-Have Features (Post v0.1.0)

| # | Feature | Priority |
|---|---|---|
| N1 | Clipboard preservation | Save/restore clipboard when injecting text |
| N2 | Cursor indicator | Floating mic icon near caret during recording |
| N3 | Translation mode | Speak in one language, output English text |
| N4 | DirectML support | ONNX Runtime + DirectML EP for DirectX 12 GPUs |
| N5 | Auto-update | Check GitHub Releases, prompt to download |
| N6 | Audio level in tray | Live audio level indicator in tray icon |
| N7 | Portable mode | Run from USB stick, no installation needed |
| N8 | MSI auto-repair | Self-healing installation |

---

## 7. Build System

### 7.1 CMake Configuration

```cmake
cmake_minimum_required(VERSION 3.21)
project(VocaWin VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Options
option(VOCAWIN_CUDA "Enable CUDA GPU acceleration" OFF)
option(VOCAWIN_VULKAN "Enable Vulkan GPU acceleration" OFF)
option(VOCAWIN_TESTS "Build tests" ON)

# vcpkg integration
find_package(vcpkg REQUIRED)

# Dependencies via vcpkg
find_package(wil CONFIG REQUIRED)           # Windows Implementation Library
find_package(nlohmann_json CONFIG REQUIRED)  # JSON parsing
find_package(GTest CONFIG REQUIRED)          # Testing

# Windows App SDK / WinUI 3
find_package(Microsoft.Windows.AppSDK REQUIRED)
find_package(Microsoft.Windows.ImplementationLibrary REQUIRED)

# whisper.cpp (submodule)
add_subdirectory(whisper.cpp)

# GPU backends for whisper.cpp
if(VOCAWIN_CUDA)
    set(GGML_CUDA ON CACHE BOOL "" FORCE)
endif()
if(VOCAWIN_VULKAN)
    set(GGML_VULKAN ON CACHE BOOL "" FORCE)
endif()

# VocaWin library (core logic)
add_library(vocawin_core STATIC
    src/app/AppController.cpp
    src/audio/AudioCapture.cpp
    src/audio/AudioBuffer.cpp
    src/audio/SilenceDetector.cpp
    src/audio/SoundFeedback.cpp
    src/speech/WhisperEngine.cpp
    src/speech/ModelManager.cpp
    src/speech/ModelInfo.cpp
    src/input/HotkeyManager.cpp
    src/input/TextInjector.cpp
    src/input/ClipboardManager.cpp
    src/platform/Autostart.cpp
    src/platform/SystemInfo.cpp
    src/platform/GpuDetector.cpp
    src/platform/Notification.cpp
    src/config/Settings.cpp
    src/config/SettingsStore.cpp
    src/util/Logger.cpp
)
target_link_libraries(vocawin_core PRIVATE
    whisper whisper-encoder-loader
    nlohmann_json::nlohmann_json
    wil::wil
)

# VocaWin WinUI 3 application
add_executable(vocawin WIN32
    src/main.cpp
    src/app/App.xaml.cpp
    src/ui/TrayIcon.cpp
    src/ui/TrayIconManager.cpp
    src/ui/SettingsWindow.xaml.cpp
    src/ui/OnboardingWindow.xaml.cpp
    src/ui/OverlayWindow.cpp
)
target_link_libraries(vocawin PRIVATE
    vocawin_core
    Microsoft::WindowsAppSDK
)

# Tests
if(VOCAWIN_TESTS)
    add_executable(vocawin_tests ...)
    target_link_libraries(vocawin_tests PRIVATE vocawin_core GTest::GTest)
endif()
```

### 7.2 Build Presets (`CMakePresets.json`)

```json
{
  "version": 6,
  "configurePresets": [
    {
      "name": "debug",
      "configurationType": "Debug",
      "cacheVariables": { "VOCAWIN_TESTS": "ON" }
    },
    {
      "name": "release",
      "configurationType": "Release"
    },
    {
      "name": "release-cuda",
      "inherits": "release",
      "cacheVariables": { "VOCAWIN_CUDA": "ON" }
    },
    {
      "name": "release-vulkan",
      "inherits": "release",
      "cacheVariables": { "VOCAWIN_VULKAN": "ON" }
    },
    {
      "name": "release-all-gpu",
      "inherits": "release",
      "cacheVariables": { "VOCAWIN_CUDA": "ON", "VOCAWIN_VULKAN": "ON" }
    }
  ]
}
```

### 7.3 Build Instructions

```powershell
# Debug build (CPU only, with tests)
cmake --preset debug
cmake --build --preset debug
ctest --preset debug

# Release build with all GPU backends
cmake --preset release-all-gpu
cmake --build --preset release-all-gpu

# Build MSI installer
cmake --build --preset release-all-gpu --target package
```

---

## 8. CI/CD Pipeline

### 8.1 CI Workflow (`ci.yml`)

```yaml
name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: windows-2022
    strategy:
      matrix:
        preset: [debug, release-all-gpu]
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - uses: actions/setup-python@v5
      - run: pip install meson ninja
      - uses: lukka/run-vcpkg@v11
      - run: cmake --preset ${{ matrix.preset }}
      - run: cmake --build --preset ${{ matrix.preset }}
      - run: ctest --preset ${{ matrix.preset }}
        if: matrix.preset == 'debug'
```

### 8.2 Release Workflow (`release.yml`)

```yaml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  release:
    runs-on: windows-2022
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - run: cmake --preset release-all-gpu
      - run: cmake --build --preset release-all-gpu
      - run: cmake --build --preset release-all-gpu --target package
      - uses: softprops/action-gh-release@v2
        with:
          files: build/release-all-gpu/*.msi
```

---

## 9. Windows-Specific Considerations

### 9.1 Permissions

- **Microphone**: Windows privacy settings control mic access. App must handle `E_ACCESSDENIED` gracefully and guide users to Settings > Privacy > Microphone.
- **No accessibility permission needed**: Unlike macOS, Windows doesn't require explicit accessibility consent for global hooks or SendInput (except UAC considerations).
- **UAC**: SendInput cannot inject text into elevated (admin) applications from a non-elevated process. Document this limitation.

### 9.2 Antivirus False Positives

- Global keyboard hooks (`SetWindowsHookEx`) and input simulation (`SendInput`) are flagged by some antivirus software.
- **Mitigation**: Code-sign the binary with an EV certificate. Add the app to Windows Defender exclusions documentation.

### 9.3 Windows Versions

- **Minimum**: Windows 10 version 1809 (17763) — required for Windows App SDK
- **Recommended**: Windows 10 21H2+ or Windows 11
- WinUI 3 requires Windows App Runtime, which the MSI installer will bundle

### 9.4 Model Storage

- **Models directory**: `%LOCALAPPDATA%\VocaWin\models\`
- **Config directory**: `%APPDATA%\VocaWin\config.json`
- **Logs directory**: `%LOCALAPPDATA%\VocaWin\logs\`
- **Rationale**: `%LOCALAPPDATA%` for large files (models), `%APPDATA%` for roaming settings

### 9.5 Autostart

- Registry key: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- Value: `VocaWin` = `"C:\Program Files\VocaWin\VocaWin.exe" --minimized`
- Or: Startup folder shortcut in `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`

---

## 10. Dependencies

### 10.1 Build Dependencies

| Dependency | Version | Source | Purpose |
|---|---|---|---|
| **whisper.cpp** | v1.8.x | Git submodule | Speech recognition engine |
| **Windows App SDK** | 1.5+ | vcpkg | WinUI 3, system tray |
| **WIL** | latest | vcpkg | Windows Implementation Library (COM helpers) |
| **nlohmann/json** | 3.x | vcpkg | JSON config parsing |
| **Google Test** | 1.14+ | vcpkg | Unit testing |
| **CUDA Toolkit** | 12.x | NVIDIA (optional) | CUDA GPU backend |
| **Vulkan SDK** | 1.3+ | LunarG (optional) | Vulkan GPU backend |
| **WiX Toolset** | v4 | NuGet | MSI installer |

### 10.2 Runtime Dependencies

- Windows App Runtime (bundled by MSI)
- Visual C++ Redistributable (statically linked to avoid dependency)
- CUDA runtime DLLs (bundled for CUDA builds, or dynamically loaded)
- Vulkan runtime (typically pre-installed with GPU drivers)

---

## 11. Performance Targets

| Metric | Target | Measurement |
|---|---|---|
| **Latency (base model, CPU)** | < 3s for 5s audio | On 8th-gen+ Intel i5 |
| **Latency (base model, CUDA)** | < 1s for 5s audio | On RTX 3060+ |
| **Latency (small model, CUDA)** | < 2s for 5s audio | On RTX 3060+ |
| **Memory (base model)** | < 500 MB | Process working set |
| **Memory (small model)** | < 1 GB | Process working set |
| **CPU idle** | < 0.1% | When not recording/processing |
| **Binary size** | < 20 MB | Without models |
| **MSI installer size** | < 25 MB | Without models |
| **Startup time** | < 2s | To tray icon visible |

---

## 12. Implementation Phases

### Phase 1: Foundation (Week 1-2)
- [ ] Project scaffolding (CMake, vcpkg, git submodule for whisper.cpp)
- [ ] `main.cpp` with WinUI 3 app initialization
- [ ] `SingleInstance` (named mutex)
- [ ] `Settings` + `SettingsStore` (JSON config)
- [ ] `Logger` (file logging with rotation)
- [ ] Basic `TrayIcon` (static icon, quit menu item)
- [ ] CI workflow (build on Windows)

### Phase 2: Core Pipeline (Week 3-4)
- [ ] `AudioCapture` (WASAPI microphone capture)
- [ ] `AudioBuffer` (lock-free ring buffer)
- [ ] `SilenceDetector` (RMS-based VAD)
- [ ] `WhisperEngine` (whisper.cpp integration, CPU first)
- [ ] `TextInjector` (SendInput Unicode)
- [ ] `ClipboardManager` (save/restore)
- [ ] `HotkeyManager` (SetWindowsHookEx)
- [ ] `AppController` (state machine, orchestration)

### Phase 3: End-to-End Flow (Week 5)
- [ ] Connect hotkey → recording → transcription → injection pipeline
- [ ] Tray icon state transitions
- [ ] Sound feedback (start/stop WAV playback)
- [ ] First successful end-to-end voice-to-text

### Phase 4: UI (Week 6-7)
- [ ] Settings window (WinUI 3 XAML, 5 tabs)
- [ ] Model management UI (download, switch, delete)
- [ ] Audio device selector with live VU meter
- [ ] Hotkey configuration UI
- [ ] Onboarding wizard (first-run flow)

### Phase 5: GPU & Polish (Week 8-9)
- [ ] CUDA backend integration and testing
- [ ] Vulkan backend integration and testing
- [ ] `GpuDetector` (hardware detection, backend selection)
- [ ] `SystemInfo` (CPU, RAM, GPU display)
- [ ] Model recommendation algorithm
- [ ] Autostart (Registry Run key)

### Phase 6: Packaging & Distribution (Week 10)
- [ ] MSI installer (WiX v4)
- [ ] Release CI workflow (build MSI on tag)
- [ ] Code signing setup (optional, self-signed for beta)
- [ ] Testing and bug fixes
- [ ] README, CONTRIBUTING.md, LICENSE
- [ ] AGENTS.md for AI-assisted development

### Phase 7: Post-Launch Enhancements
- [ ] Auto-update (GitHub Releases API)
- [ ] Cursor indicator (floating mic overlay)
- [ ] DirectML support (ONNX Runtime)
- [ ] Translation mode
- [ ] Microsoft Store (MSIX packaging)

---

## 13. Open Questions

| # | Question | Options | Recommendation |
|---|---|---|---|
| Q1 | **License?** | GPL-3.0 (like VocaLinux) or AGPL-3.0 (like VocaMac) | AGPL-3.0 for consistency with newer Voca apps |
| Q2 | **Ship one binary or multiple GPU variants?** | Single binary with all backends compiled in vs. separate CUDA/Vulkan/CPU downloads | Single binary with runtime detection. Simpler distribution. |
| Q3 | **Code signing?** | Self-signed (free, triggers SmartScreen warning) vs. EV certificate (~$300-500/year, trusted) | Start self-signed for beta. Get EV cert for v1.0. |
| Q4 | **Audio library?** | Raw WASAPI vs. miniaud.io (single-header) vs. SDL2 | Raw WASAPI for zero dependencies and full control. Miniaudio as fallback if WASAPI is too complex. |
| Q5 | **First model download** | Bundle tiny model in MSI (~75MB larger) vs. download on first run | Download on first run (smaller installer, onboarding wizard handles it). |

---

## 14. Risks and Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| **WinUI 3 complexity** — Steep learning curve, XAML Islands, WinRT ABI | High | Consider falling back to plain Win32 + Direct2D for settings UI if WinUI 3 proves too complex. |
| **WASAPI audio capture complexity** | Medium | Use miniaudio (single-header, MIT) as fallback. Supports WASAPI internally. |
| **SendInput blocked by UAC elevation mismatch** | Medium | Document the limitation. Run as same elevation as target apps. Provide clipboard-paste as alternative. |
| **whisper.cpp GPU build fragility** | Medium | Offer CPU-only build as default. GPU builds as opt-in. Test on variety of hardware. |
| **Antivirus false positives** | High | Code-sign with EV cert. Provide clear docs. Submit to Windows Defender whitelist. |
| **WinUI 3 unpackaged app limitations** | Medium | Windows App SDK 1.5+ supports unpackaged (non-MSIX) distribution with self-contained runtime. Test this path early. |
| **Large binary size with all GPU backends** | Low | ~20-30MB is acceptable for a desktop app. Not a concern. |
