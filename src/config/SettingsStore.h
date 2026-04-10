#pragma once

#include <filesystem>

#include "config/Settings.h"

namespace vocawin {

class SettingsStore {
public:
    explicit SettingsStore(std::filesystem::path config_path);

    Settings load() const;
    bool save(const Settings& settings) const;

private:
    std::filesystem::path config_path_;
};

}  // namespace vocawin
