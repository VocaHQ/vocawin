#include "util/Logger.h"

#include <chrono>
#include <fstream>
#include <iomanip>
#include <sstream>

namespace vocawin {

namespace {

std::string nowIso8601() {
    const auto now = std::chrono::system_clock::now();
    const auto time = std::chrono::system_clock::to_time_t(now);

    std::tm tm{};
#if defined(_WIN32)
    gmtime_s(&tm, &time);
#else
    gmtime_r(&time, &tm);
#endif

    std::ostringstream ss;
    ss << std::put_time(&tm, "%Y-%m-%dT%H:%M:%SZ");
    return ss.str();
}

}  // namespace

Logger::Logger(std::filesystem::path log_path) : log_path_(std::move(log_path)) {}

bool Logger::initialize() {
    const auto parent = log_path_.parent_path();
    if (!parent.empty()) {
        std::error_code ec;
        std::filesystem::create_directories(parent, ec);
        if (ec) {
            return false;
        }
    }

    std::ofstream out(log_path_, std::ios::app);
    if (!out.is_open()) {
        return false;
    }

    out << nowIso8601() << " [INFO] logger initialized\n";
    return true;
}

void Logger::info(const std::string& message) {
    write("INFO", message);
}

void Logger::error(const std::string& message) {
    write("ERROR", message);
}

void Logger::write(const std::string& level, const std::string& message) {
    std::lock_guard<std::mutex> lock(mutex_);

    std::ofstream out(log_path_, std::ios::app);
    if (!out.is_open()) {
        return;
    }

    out << nowIso8601() << " [" << level << "] " << message << "\n";
}

}  // namespace vocawin
