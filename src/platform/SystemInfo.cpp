#include "platform/SystemInfo.h"

#include <cstring>
#include <sstream>

#if defined(_WIN32)
#include <windows.h>
#include <intrin.h>
#endif

namespace vocawin {

std::string SystemInfo::cpuName() {
#if defined(_WIN32)
    int cpuInfo[4] = {0};
    char brand[49] = {0};
    __cpuid(cpuInfo, 0x80000000);
    const unsigned int maxExt = static_cast<unsigned int>(cpuInfo[0]);
    if (maxExt >= 0x80000004) {
        __cpuid(cpuInfo, 0x80000002);
        std::memcpy(brand, cpuInfo, 16);
        __cpuid(cpuInfo, 0x80000003);
        std::memcpy(brand + 16, cpuInfo, 16);
        __cpuid(cpuInfo, 0x80000004);
        std::memcpy(brand + 32, cpuInfo, 16);
        brand[48] = '\0';
    }
    return std::string(brand);
#else
    return "Unknown CPU";
#endif
}

std::string SystemInfo::osName() {
#if defined(_WIN32)
    OSVERSIONINFOEXW osvi{};
    osvi.dwOSVersionInfoSize = sizeof(osvi);
    typedef LONG(WINAPI* RtlGetVersionPtr)(PRTL_OSVERSIONINFOW);
    HMODULE hMod = GetModuleHandleW(L"ntdll.dll");
    if (hMod) {
        auto fx = reinterpret_cast<RtlGetVersionPtr>(
            GetProcAddress(hMod, "RtlGetVersion"));
        if (fx) {
            RTL_OSVERSIONINFOW rovi{};
            rovi.dwOSVersionInfoSize = sizeof(rovi);
            if (fx(&rovi) == 0) {
                std::ostringstream os;
                os << "Windows " << rovi.dwMajorVersion << "."
                   << rovi.dwMinorVersion << " (build " << rovi.dwBuildNumber
                   << ")";
                return os.str();
            }
        }
    }
    return "Windows";
#else
    return "Unknown OS";
#endif
}

std::size_t SystemInfo::totalRamBytes() {
#if defined(_WIN32)
    MEMORYSTATUSEX statex{};
    statex.dwLength = sizeof(statex);
    if (GlobalMemoryStatusEx(&statex)) {
        return static_cast<std::size_t>(statex.ullTotalPhys);
    }
    return 0;
#else
    return 0;
#endif
}

std::string SystemInfo::summary() {
    std::ostringstream os;
    os << cpuName() << " | " << osName() << " | RAM: "
       << (totalRamBytes() / (1024 * 1024)) << " MB";
    return os.str();
}

}  // namespace vocawin
