#pragma once

#include <filesystem>
#include <mutex>
#include <string>

namespace vocawin {

class Logger {
public:
    explicit Logger(std::filesystem::path log_path);

    bool initialize();
    void info(const std::string& message);
    void error(const std::string& message);

private:
    void write(const std::string& level, const std::string& message);

    std::filesystem::path log_path_;
    std::mutex mutex_;
};

}  // namespace vocawin
