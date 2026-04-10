#include "app/AppController.h"

namespace vocawin {

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
