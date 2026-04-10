#include "app/SingleInstance.h"

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

SingleInstance::SingleInstance(std::wstring name) : name_(std::move(name)) {}

SingleInstance::~SingleInstance() {
#if defined(_WIN32)
    if (handle_ != nullptr) {
        CloseHandle(static_cast<HANDLE>(handle_));
        handle_ = nullptr;
    }
#endif
}

bool SingleInstance::acquire() {
#if defined(_WIN32)
    handle_ = CreateMutexW(nullptr, FALSE, name_.c_str());
    if (handle_ == nullptr) {
        has_lock_ = false;
        return false;
    }

    if (GetLastError() == ERROR_ALREADY_EXISTS) {
        has_lock_ = false;
        return false;
    }

    has_lock_ = true;
#else
    has_lock_ = true;
#endif
    return has_lock_;
}

bool SingleInstance::hasLock() const {
    return has_lock_;
}

}  // namespace vocawin
