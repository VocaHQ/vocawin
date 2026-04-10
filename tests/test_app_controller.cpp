#include <cassert>
#include <filesystem>
#include <fstream>
#include <string>

#include "app/AppController.h"

int main() {
    const std::filesystem::path root = "build/test-app-controller";
    std::filesystem::remove_all(root);

    {
        // Existing config path: initialize should load custom values.
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

    {
        vocawin::AppController app1(root);
        const bool initialized1 = app1.initialize();
        assert(initialized1);

        vocawin::AppController app2(root);
        const bool initialized2 = app2.initialize();
#if defined(_WIN32)
        assert(!initialized2);
#else
        assert(initialized2);
        app2.shutdown();
#endif

        app1.shutdown();
    }

    std::filesystem::remove_all(root);
    return 0;
}
