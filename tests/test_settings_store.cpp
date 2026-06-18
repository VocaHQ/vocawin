#include <cassert>
#include <filesystem>
#include <fstream>

#include "config/SettingsStore.h"

int main() {
    const std::filesystem::path root = "build/test-settings-store";
    const std::filesystem::path path = root / "test-settings.json";
    std::filesystem::remove_all(root);

    // Missing file should return defaults.
    {
        vocawin::SettingsStore store(path);
        const vocawin::Settings defaults = store.load();
        assert(defaults.model_id == "base.en");
        assert(defaults.language == "auto");
        assert(defaults.launch_at_startup);
        assert(defaults.hotkey_vk_code == 0xA3);  // VK_RCONTROL
        assert(defaults.activation_mode == 0);
        assert(defaults.silence_threshold == 0.01f);
        assert(defaults.text_injection_method == 0);
    }

    // Save + load happy path.
    {
        vocawin::SettingsStore store(path);
        vocawin::Settings settings;
        settings.model_id = "tiny";
        settings.language = "en";
        settings.launch_at_startup = false;
        settings.sound_effects = false;
        settings.preserve_clipboard = false;

        const bool saved = store.save(settings);
        assert(saved);

        const vocawin::Settings loaded = store.load();
        assert(loaded.model_id == "tiny");
        assert(loaded.language == "en");
        assert(!loaded.launch_at_startup);
        assert(!loaded.sound_effects);
        assert(!loaded.preserve_clipboard);
    }

    // Invalid bool values should fall back to defaults.
    {
        const std::filesystem::path malformed_path = root / "malformed.json";
        std::ofstream out(malformed_path);
        out << "{\n";
        out << "  \"modelId\": \"small\",\n";
        out << "  \"language\": \"fr\",\n";
        out << "  \"launchAtStartup\": maybe,\n";
        out << "  \"soundEffects\": maybe,\n";
        out << "  \"preserveClipboard\": maybe\n";
        out << "}\n";
        out.close();

        vocawin::SettingsStore store(malformed_path);
        const vocawin::Settings loaded = store.load();
        assert(loaded.model_id == "small");
        assert(loaded.language == "fr");
        assert(loaded.launch_at_startup);
        assert(loaded.sound_effects);
        assert(loaded.preserve_clipboard);
    }

    // Save failure path: parent is a file, not a directory.
    {
        const std::filesystem::path blocker = root / "blocker";
        std::ofstream out(blocker);
        out << "x";
        out.close();

        vocawin::SettingsStore store(blocker / "nested" / "config.json");
        const vocawin::Settings settings;
        const bool saved = store.save(settings);
        assert(!saved);
    }

    std::filesystem::remove_all(root);

    return 0;
}
