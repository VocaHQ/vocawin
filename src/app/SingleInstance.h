#pragma once

#include <cstdint>
#include <string>

namespace vocawin {

class SingleInstance {
public:
    explicit SingleInstance(std::wstring name);
    ~SingleInstance();

    bool acquire();
    bool hasLock() const;

private:
    std::wstring name_;
    bool has_lock_{false};
#if defined(_WIN32)
    void* handle_{nullptr};
#endif
};

}  // namespace vocawin
