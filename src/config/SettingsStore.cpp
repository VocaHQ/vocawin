#include "config/SettingsStore.h"

#include <algorithm>
#include <cstdint>
#include <fstream>
#include <regex>
#include <sstream>
#include <string>
#include <string_view>

namespace {

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

long long extractInt(const std::string& json, const std::string& key, long long fallback) {
    const std::regex pattern("\"" + key + "\"\\s*:\\s*(-?\\d+)");
    std::smatch match;
    if (std::regex_search(json, match, pattern) && match.size() == 2) {
        try {
            return std::stoll(match[1].str());
        } catch (...) {
            return fallback;
        }
    }
    return fallback;
}

double extractDouble(const std::string& json, const std::string& key, double fallback) {
    const std::regex pattern("\"" + key + "\"\\s*:\\s*([+-]?\\d+(?:\\.\\d+)?)");
    std::smatch match;
    if (std::regex_search(json, match, pattern) && match.size() == 2) {
        try {
            return std::stod(match[1].str());
        } catch (...) {
            return fallback;
        }
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

    settings.hotkey_vk_code = static_cast<std::uint32_t>(
        extractInt(json, "hotkeyVkCode", static_cast<long long>(settings.hotkey_vk_code)));
    settings.activation_mode = static_cast<int>(
        extractInt(json, "activationMode", settings.activation_mode));
    settings.double_tap_threshold_ms = extractDouble(
        json, "doubleTapThresholdMs", settings.double_tap_threshold_ms);
    settings.silence_threshold = static_cast<float>(extractDouble(
        json, "silenceThreshold", settings.silence_threshold));
    settings.silence_duration_ms = static_cast<std::uint32_t>(extractInt(
        json, "silenceDurationMs", static_cast<long long>(settings.silence_duration_ms)));
    settings.max_recording_duration_s = static_cast<std::uint32_t>(extractInt(
        json, "maxRecordingDurationS", static_cast<long long>(settings.max_recording_duration_s)));
    settings.text_injection_method = static_cast<int>(extractInt(
        json, "textInjectionMethod", settings.text_injection_method));
    settings.paste_delay_ms = static_cast<std::uint32_t>(extractInt(
        json, "pasteDelayMs", static_cast<long long>(settings.paste_delay_ms)));
    settings.restore_delay_ms = static_cast<std::uint32_t>(extractInt(
        json, "restoreDelayMs", static_cast<long long>(settings.restore_delay_ms)));
    settings.models_dir = extractString(json, "modelsDir", settings.models_dir);

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
    out << "  \"preserveClipboard\": " << (settings.preserve_clipboard ? "true" : "false") << ",\n";
    out << "  \"hotkeyVkCode\": " << settings.hotkey_vk_code << ",\n";
    out << "  \"activationMode\": " << settings.activation_mode << ",\n";
    out << "  \"doubleTapThresholdMs\": " << settings.double_tap_threshold_ms << ",\n";
    out << "  \"silenceThreshold\": " << settings.silence_threshold << ",\n";
    out << "  \"silenceDurationMs\": " << settings.silence_duration_ms << ",\n";
    out << "  \"maxRecordingDurationS\": " << settings.max_recording_duration_s << ",\n";
    out << "  \"textInjectionMethod\": " << settings.text_injection_method << ",\n";
    out << "  \"pasteDelayMs\": " << settings.paste_delay_ms << ",\n";
    out << "  \"restoreDelayMs\": " << settings.restore_delay_ms << ",\n";
    out << "  \"modelsDir\": \"" << settings.models_dir << "\"\n";
    out << "}\n";

    return true;
}

}  // namespace vocawin
