#pragma once

#include <filesystem>

#include "app/SingleInstance.h"
#include "config/Settings.h"
#include "config/SettingsStore.h"
#include "ui/TrayIcon.h"
#include "util/Logger.h"

namespace vocawin {

class AppController {
public:
    explicit AppController(std::filesystem::path data_root = "vocawin");

    bool initialize();
    void shutdown();

    bool isInitialized() const;
    const Settings& settings() const;

private:
    std::filesystem::path data_root_;
    bool initialized_{false};
    SingleInstance single_instance_;
    SettingsStore settings_store_;
    Settings settings_{};
    TrayIcon tray_icon_{};
    Logger logger_;
};

}  // namespace vocawin
