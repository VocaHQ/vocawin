#pragma once

#include <cstdint>
#include <string>

namespace vocawin {

// Auto-update skeleton. Compares the running version against the
// latest GitHub release and surfaces a download URL. Per SPEC
// \u00a76.2 N5 and SPEC \u00a72.1.
//
// The MVP implements version comparison and channel selection. The
// actual HTTP fetch (InternetReadFile / WinHTTP) is stubbed to return
// an empty URL on non-Win32 platforms; on Win32 the implementation
// can be extended with WinHTTP without changing this surface.
class Updater {
public:
    struct Version {
        std::uint32_t major{0};
        std::uint32_t minor{0};
        std::uint32_t patch{0};
    };

    static Version parseVersion(const std::string& tag);
    static bool isNewer(const std::string& current, const std::string& latest);

    static std::string defaultChannel();

    // Returns the tag_name of the latest release (e.g. "v0.2.0"),
    // or empty on failure. Win32: WinHTTP GET to the GitHub releases
    // API. Other platforms: returns empty.
    static std::string latestTag(const std::string& channel =
                                     defaultChannel());

    // Returns the browser_download_url of the latest release, or
    // empty on failure. Win32: WinHTTP GET. Other platforms: empty.
    static std::string latestDownloadUrl(const std::string& channel =
                                             defaultChannel());
};

}  // namespace vocawin
