#include "config/SettingsStore.h"

#include <algorithm>
#include <fstream>
#include <regex>
#include <string>
#include <string_view>

namespace {

std::string trim(std::string value) {
    value.erase(value.begin(), std::find_if(value.begin(), value.end(), [](unsigned char c) {
        return !std::isspace(c);
    }));

    value.erase(std::find_if(value.rbegin(), value.rend(), [](unsigned char c) {
        return !std::isspace(c);
    }).base(), value.end());

    return value;
}

bool toBool(std::string_view value, bool fallback) {
    if (value == "true") {
        return true;
    }
    if (value == "false") {
        return false;
    }
    return fallback;
}

std::string extractString(const std::string& json, const std::string& key, const std::string& fallback) {
    const std::regex pattern("\"" + key + "\"\\s*:\\s*\"([^\"]*)\"");
    std::smatch match;
    if (std::regex_search(json, match, pattern) && match.size() == 2) {
        return match[1].str();
    }
    return fallback;
}

bool extractBool(const std::string& json, const std::string& key, bool fallback) {
    const std::regex pattern("\"" + key + "\"\\s*:\\s*(true|false)");
    std::smatch match;
    if (std::regex_search(json, match, pattern) && match.size() == 2) {
        return toBool(match[1].str(), fallback);
    }
    return fallback;
}

}  // namespace

namespace vocawin {

SettingsStore::SettingsStore(std::filesystem::path config_path)
    : config_path_(std::move(config_path)) {}

Settings SettingsStore::load() const {
    Settings settings;

    if (!std::filesystem::exists(config_path_)) {
        return settings;
    }

    std::ifstream in(config_path_);
    if (!in.is_open()) {
        return settings;
    }

    const std::string json((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());

    settings.model_id = extractString(json, "modelId", settings.model_id);
    settings.language = extractString(json, "language", settings.language);
    settings.launch_at_startup = extractBool(json, "launchAtStartup", settings.launch_at_startup);
    settings.sound_effects = extractBool(json, "soundEffects", settings.sound_effects);
    settings.preserve_clipboard = extractBool(json, "preserveClipboard", settings.preserve_clipboard);

    return settings;
}

bool SettingsStore::save(const Settings& settings) const {
    auto parent = config_path_.parent_path();
    if (!parent.empty()) {
        std::error_code ec;
        std::filesystem::create_directories(parent, ec);
        if (ec) {
            return false;
        }
    }

    std::ofstream out(config_path_);
    if (!out.is_open()) {
        return false;
    }

    out << "{\n";
    out << "  \"modelId\": \"" << settings.model_id << "\",\n";
    out << "  \"language\": \"" << settings.language << "\",\n";
    out << "  \"launchAtStartup\": " << (settings.launch_at_startup ? "true" : "false") << ",\n";
    out << "  \"soundEffects\": " << (settings.sound_effects ? "true" : "false") << ",\n";
    out << "  \"preserveClipboard\": " << (settings.preserve_clipboard ? "true" : "false") << "\n";
    out << "}\n";

    return true;
}

}  // namespace vocawin
