#include <cassert>
#include <filesystem>
#include <fstream>
#include <string>

#include "util/Logger.h"

int main() {
    const std::filesystem::path root = "build/test-logger";
    const std::filesystem::path file = root / "vocawin.log";
    std::filesystem::remove_all(root);

    vocawin::Logger logger(file);
    assert(logger.initialize());

    logger.info("hello");
    logger.error("boom");

    std::ifstream in(file);
    assert(in.is_open());

    std::string all((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
    assert(all.find("logger initialized") != std::string::npos);
    assert(all.find("[INFO] hello") != std::string::npos);
    assert(all.find("[ERROR] boom") != std::string::npos);

    // Write path not open case should be safely ignored.
    const std::filesystem::path bad_file = root;
    vocawin::Logger bad_logger(bad_file);
    const bool bad_init = bad_logger.initialize();
    assert(!bad_init);
    bad_logger.info("ignored");
    bad_logger.error("ignored");

    std::filesystem::remove_all(root);
    return 0;
}
