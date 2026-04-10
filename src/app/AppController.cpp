#include "app/AppController.h"

namespace vocawin {

namespace {

std::wstring mutexNameFromPath(const std::filesystem::path& path) {
    const auto value = path.string();
    return std::wstring(value.begin(), value.end());
}

}  // namespace

AppController::AppController(std::filesystem::path data_root)
    : data_root_(std::move(data_root)),
      single_instance_(L"Global\\VocaWinMutex-" + mutexNameFromPath(data_root_)),
      settings_store_(data_root_ / "config.json"),
      logger_(data_root_ / "logs" / "vocawin.log") {}

bool AppController::initialize() {
    if (!single_instance_.acquire()) {
        return false;
    }

    if (!logger_.initialize()) {
        return false;
    }

    settings_ = settings_store_.load();
    logger_.info("settings loaded");

    if (!tray_icon_.initialize()) {
        logger_.error("failed to initialize tray icon");
        return false;
    }

    initialized_ = true;
    return true;
}

void AppController::shutdown() {
    if (!initialized_) {
        return;
    }

    logger_.info("shutdown complete");
    tray_icon_.shutdown();
    initialized_ = false;
}

bool AppController::isInitialized() const {
    return initialized_;
}

const Settings& AppController::settings() const {
    return settings_;
}

}  // namespace vocawin
