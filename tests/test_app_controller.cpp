#include <cassert>
#include <filesystem>
#include <fstream>
#include <string>

#include "app/AppController.h"

namespace {

void writeCustomConfig(const std::filesystem::path& root) {
    std::filesystem::create_directories(root);
    std::ofstream out(root / "config.json");
    out << "{\n";
    out << "  \"modelId\": \"small\",\n";
    out << "  \"language\": \"de\",\n";
    out << "  \"launchAtStartup\": false,\n";
    out << "  \"soundEffects\": false,\n";
    out << "  \"preserveClipboard\": false\n";
    out << "}\n";
    out.close();
}

}  // namespace

int main() {
    const std::filesystem::path root = "build/test-app-controller";
    std::filesystem::remove_all(root);

    // 1. Custom config path: initialize should load custom values.
    {
        writeCustomConfig(root);
        vocawin::AppController app(root);
        const bool initialized = app.initialize();
        assert(initialized);
        assert(app.isInitialized());

        const auto& settings = app.settings();
        assert(settings.model_id == "small");
        assert(settings.language == "de");
        assert(!settings.launch_at_startup);
        assert(!settings.sound_effects);
        assert(!settings.preserve_clipboard);

        app.shutdown();
        assert(!app.isInitialized());
    }

    // 2. Single-instance guard.
    {
        vocawin::AppController app1(root);
        assert(app1.initialize());
        vocawin::AppController app2(root);
        const bool init2 = app2.initialize();
#if defined(_WIN32)
        assert(!init2);
#else
        assert(init2);
        app2.shutdown();
#endif
        app1.shutdown();
    }

    // 3. State before init is NotLoaded.
    {
        vocawin::AppController app(root);
        assert(app.state() == vocawin::AppController::State::NotLoaded);
    }

    // 4. State after init is NotLoaded (no model downloaded in this test env).
    {
        writeCustomConfig(root);
        vocawin::AppController app(root);
        assert(app.initialize());
        // Either Idle (model found) or NotLoaded (model missing) is valid.
        const auto s = app.state();
        assert(s == vocawin::AppController::State::Idle ||
               s == vocawin::AppController::State::NotLoaded);
    }

    // 5. startRecording before init: no crash, state unchanged.
    {
        vocawin::AppController app(root);
        const auto before = app.state();
        app.startRecording();
        assert(app.state() == before);
    }

    // 6. cancelRecording on a fresh controller: no crash, state unchanged.
    {
        vocawin::AppController app(root);
        const auto before = app.state();
        app.cancelRecording();
        assert(app.state() == before);
    }

    // 7. stopRecordingAndTranscribe without model: no crash.
    {
        writeCustomConfig(root);
        vocawin::AppController app(root);
        app.initialize();
        app.stopRecordingAndTranscribe();
        app.shutdown();
    }

    // 8. onStateChanged callback can be registered without crash.
    {
        writeCustomConfig(root);
        vocawin::AppController app(root);
        int callbackCount = 0;
        app.onStateChanged = [&callbackCount](vocawin::AppController::State) {
            ++callbackCount;
        };
        app.initialize();
        (void)callbackCount;
        app.shutdown();
    }

    // 9. lastError() starts empty.
    {
        vocawin::AppController app(root);
        assert(app.lastError().empty());
    }

    // 10. downloadModel with unknown id returns false (no network call).
    {
        writeCustomConfig(root);
        vocawin::AppController app(root);
        assert(app.initialize());
        assert(!app.downloadModel("nonexistent-model-id"));
        app.shutdown();
    }

    // 11. downloadModel with valid id but non-existent file:// URL returns
    //     false; onProgress must not fire.
    {
        writeCustomConfig(root);
        vocawin::AppController app(root);
        assert(app.initialize());
        bool progressFired = false;
        app.setDownloadProgressHandler([&progressFired](float) {
            progressFired = true;
        });
        assert(!app.downloadModel("base.en"));
        assert(!progressFired);
        app.shutdown();
    }

    // 12. settingsWindow() accessor returns a valid reference.
    {
        writeCustomConfig(root);
        vocawin::AppController app(root);
        app.initialize();
        auto& sw = app.settingsWindow();
        (void)sw;  // smoke: accessor compiles + returns non-null ref
        app.shutdown();
    }

    std::filesystem::remove_all(root);
    return 0;
}
