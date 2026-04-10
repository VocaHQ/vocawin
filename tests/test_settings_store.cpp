#include <cassert>
#include <filesystem>

#include "config/SettingsStore.h"

int main() {
    const std::filesystem::path path = "build/test-settings.json";
    vocawin::SettingsStore store(path);

    vocawin::Settings settings;
    settings.model_id = "tiny";
    settings.language = "en";
    settings.launch_at_startup = false;

    const bool saved = store.save(settings);
    assert(saved);

    const vocawin::Settings loaded = store.load();
    assert(loaded.model_id == "tiny");
    assert(loaded.language == "en");
    assert(!loaded.launch_at_startup);

    return 0;
}
