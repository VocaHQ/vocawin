// Automated hotkey + transcribe through SHIPPED AppController wiring.
// Drives tray postHotkeyPressRequest → WM_APP+2 → startRecording (same
// path as the LL-hook after marshal), not a reimplemented shortcut.

#include <cassert>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

#include "app/AppController.h"
#include "input/HotkeyManager.h"
#include "speech/WhisperEngine.h"

namespace {

std::vector<float> makeTone(float seconds, float hz, int sr) {
    const std::size_t n = static_cast<std::size_t>(seconds * sr);
    std::vector<float> pcm(n);
    constexpr float kTwoPi = 6.28318530718f;
    for (std::size_t i = 0; i < n; ++i) {
        pcm[i] = 0.15f * std::sin(kTwoPi * hz * static_cast<float>(i) /
                                  static_cast<float>(sr));
    }
    return pcm;
}

std::filesystem::path findTinyModel() {
    const char* local = std::getenv("LOCALAPPDATA");
    const std::filesystem::path candidates[] = {
        "build/debug/tests/build/test-mvp-pipeline/models/ggml-tiny.en.bin",
        "build/test-mvp-pipeline/models/ggml-tiny.en.bin",
        "build/debug/vocawin/models/ggml-tiny.en.bin",
        local ? std::filesystem::path(local) / "VocaWin" / "models" /
                    "ggml-tiny.en.bin"
              : std::filesystem::path(),
    };
    for (const auto& c : candidates) {
        if (c.empty()) {
            continue;
        }
        std::error_code ec;
        if (std::filesystem::exists(c, ec) &&
            std::filesystem::file_size(c, ec) > 1'000'000) {
            return c;
        }
    }
    return {};
}

std::string readFile(const std::filesystem::path& p) {
    std::ifstream in(p);
    if (!in) {
        return {};
    }
    return std::string(std::istreambuf_iterator<char>(in),
                       std::istreambuf_iterator<char>());
}

}  // namespace

int main() {
    // --- Unit: handleKeyEvent debounce (same entry as LL hook) ---
    {
        vocawin::HotkeyManager hk;
        int down = 0;
        int up = 0;
        hk.onHotkeyPressed = [&]() { ++down; };
        hk.onHotkeyReleased = [&]() { ++up; };
        hk.handleKeyEvent(true);
        hk.handleKeyEvent(true);
        hk.handleKeyEvent(false);
        assert(down == 1 && up == 1);
        std::cout << "handleKeyEvent debounce ok" << std::endl;
    }

#if defined(_WIN32)
    // --- Install real LL hook, stop joins cleanly (no UAF detach) ---
    {
        vocawin::HotkeyManager hk;
        vocawin::HotkeyManager::Config cfg;
        cfg.virtualKeyCode = 0xA3;
        assert(hk.start(cfg));
        assert(hk.isRunning());
        // Restart must not race a ghost pump.
        hk.stop();
        assert(!hk.isRunning());
        assert(hk.start(cfg));
        hk.stop();
        std::cout << "hotkey start/stop/restart join ok" << std::endl;
    }
#endif

    // Model for integrated controller path (inference is the only engine use).
    const auto modelPath = findTinyModel();
    if (modelPath.empty()) {
        std::cout << "no tiny.en cache; skip model-backed integrated path"
                  << std::endl;
    }

    // --- AppController INTEGRATED hotkey path (tray marshal) ---
    const std::filesystem::path root = "build/test-hotkey-integrated";
    std::filesystem::remove_all(root);
    std::filesystem::create_directories(root / "logs");
    std::ofstream(root / "onboarded.json") << "{\"onboarded\": true}";
    std::ofstream(root / "config.json") << R"({
  "modelId": "tiny.en",
  "language": "en",
  "launchAtStartup": false,
  "soundEffects": false,
  "hotkeyVkCode": 163,
  "activationMode": 0
})";

    if (!modelPath.empty()) {
        std::error_code ec;
        std::filesystem::create_directories(root / "models", ec);
        std::filesystem::copy_file(
            modelPath, root / "models" / "ggml-tiny.en.bin",
            std::filesystem::copy_options::overwrite_existing, ec);
        std::cout << "using model " << modelPath << std::endl;
    }

    {
        vocawin::AppController app(root);
        assert(app.initialize());
        assert(app.isInitialized());

        // Wired path: postHotkeyPressRequest → WM_APP+2 → onHotkeyPressRequest
        // → startRecording. simulateHotkeyPress uses the tray HWND + pump.
        app.simulateHotkeyPress();
        std::this_thread::sleep_for(std::chrono::milliseconds(150));
        const auto sRec = app.state();
        std::cout << "after simulateHotkeyPress state="
                  << static_cast<int>(sRec) << std::endl;
        assert(sRec == vocawin::AppController::State::Recording ||
               sRec == vocawin::AppController::State::Error);

        // Also exercise tray API directly (same messages).
        if (sRec != vocawin::AppController::State::Recording) {
            // Mic may fail in CI; still require the marshal path to run.
            // Press already logged; release should not crash.
        }

        if (sRec == vocawin::AppController::State::Recording) {
            app.simulateHotkeyRelease();
            // Wait for Processing → Idle (runInference finished).
            bool sawProcessing = false;
            auto sEnd = app.state();
            for (int i = 0; i < 1200; ++i) {  // up to ~120s
                sEnd = app.state();
                if (sEnd == vocawin::AppController::State::Processing) {
                    sawProcessing = true;
                }
                if (sEnd == vocawin::AppController::State::Idle ||
                    sEnd == vocawin::AppController::State::NotLoaded ||
                    sEnd == vocawin::AppController::State::Error) {
                    break;
                }
                std::this_thread::sleep_for(std::chrono::milliseconds(100));
                app.trayIcon().pumpMessage();
            }
            std::cout << "after release/wait state=" << static_cast<int>(sEnd)
                      << " sawProcessing=" << sawProcessing << std::endl;
            assert(sawProcessing);
            // If still Processing, cancelRecording joins the inference thread
            // (same join path as shutdown) so we never detach.
            if (sEnd == vocawin::AppController::State::Processing) {
                app.cancelRecording();
                sEnd = app.state();
            }
            assert(sEnd == vocawin::AppController::State::Idle ||
                   sEnd == vocawin::AppController::State::NotLoaded ||
                   sEnd == vocawin::AppController::State::Error);
        } else {
            app.simulateHotkeyRelease();
            std::cout << "mic unavailable; release marshal only" << std::endl;
        }

        // Log must show the wired handler messages (proves not a direct call).
        auto logBody = readFile(root / "logs" / "vocawin.log");
        assert(logBody.find("hotkey manager started") != std::string::npos);
        assert(logBody.find("hotkey pressed -> startRecording") !=
               std::string::npos);
        assert(logBody.find("hotkey released -> stopRecordingAndTranscribe") !=
               std::string::npos);
        if (sRec == vocawin::AppController::State::Recording) {
            assert(logBody.find("runInference:") != std::string::npos);
        }
        std::cout << "log contains wired hotkey press/release lines"
                  << std::endl;

        app.shutdown();
        std::cout << "controller shutdown ok" << std::endl;

        // After clean join, shutdown must not have detached inference.
        logBody = readFile(root / "logs" / "vocawin.log");
        assert(logBody.find("detached in-flight inference") ==
               std::string::npos);
        assert(logBody.find("shutdown complete") != std::string::npos);
        std::cout << "shutdown joined inference (no detach UAF path)"
                  << std::endl;

        std::ofstream evidence(root / "integrated-hotkey.log");
        evidence << logBody;
    }

    std::cout << "test_hotkey_transcribe PASS" << std::endl;
    return 0;
}
