#pragma once

#include "app/SingleInstance.h"
#include "config/Settings.h"
#include "config/SettingsStore.h"
#include "ui/TrayIcon.h"
#include "util/Logger.h"

namespace vocawin {

class AppController {
public:
    bool initialize();
    void shutdown();

    bool isInitialized() const;
    const Settings& settings() const;

private:
    bool initialized_{false};
    SingleInstance single_instance_{L"Global\\VocaWinMutex"};
    SettingsStore settings_store_{"vocawin/config.json"};
    Settings settings_{};
    TrayIcon tray_icon_{};
    Logger logger_{"vocawin/logs/vocawin.log"};
};

}  // namespace vocawin
