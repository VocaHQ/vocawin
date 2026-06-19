#include "updater/Updater.h"

#include <algorithm>
#include <cctype>
#include <sstream>
#include <string>
#include <vector>

#if defined(_WIN32)
#include <windows.h>
#include <winhttp.h>
#pragma comment(lib, "winhttp.lib")
#endif

namespace vocawin {

namespace {
std::string stripPrefix(const std::string& s, const std::string& prefix) {
    if (s.size() < prefix.size()) return s;
    if (s.compare(0, prefix.size(), prefix) != 0) return s;
    return s.substr(prefix.size());
}

std::string extractTagName(const std::string& json) {
    const std::string key = "\"tag_name\":\"";
    auto pos = json.find(key);
    if (pos == std::string::npos) return "";
    pos += key.size();
    auto end = json.find('"', pos);
    if (end == std::string::npos) return "";
    return json.substr(pos, end - pos);
}

std::string extractDownloadUrl(const std::string& json) {
    const std::string key = "\"browser_download_url\":\"";
    auto pos = json.find(key);
    if (pos == std::string::npos) return "";
    pos += key.size();
    auto end = json.find('"', pos);
    if (end == std::string::npos) return "";
    return json.substr(pos, end - pos);
}

#if defined(_WIN32)
std::string httpGet(const std::string& url) {
    HINTERNET session = WinHttpOpen(L"VocaWin/0.1",
        WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
        WINHTTP_NO_PROXY_NAME,
        WINHTTP_NO_PROXY_BYPASS, 0);
    if (session == nullptr) return "";
    std::wstring wurl(url.begin(), url.end());

    DWORD componentsSize = 0;
    WinHttpCrackUrl(wurl.c_str(), static_cast<DWORD>(wurl.size()), 0, nullptr);
    URL_COMPONENTS uc{};
    uc.dwStructSize = sizeof(uc);
    wchar_t hostName[256] = {};
    wchar_t urlPath[2048] = {};
    uc.lpszHostName = hostName;
    uc.dwHostNameLength = 256;
    uc.lpszUrlPath = urlPath;
    uc.dwUrlPathLength = 2048;
    if (!WinHttpCrackUrl(wurl.c_str(), static_cast<DWORD>(wurl.size()),
                         0, &uc)) {
        WinHttpCloseHandle(session);
        return "";
    }

    HINTERNET connect = WinHttpConnect(session, hostName, uc.nPort, 0);
    if (connect == nullptr) {
        WinHttpCloseHandle(session);
        return "";
    }
    DWORD flags = (uc.nScheme == INTERNET_SCHEME_HTTPS)
                      ? WINHTTP_FLAG_SECURE : 0;
    HINTERNET request = WinHttpOpenRequest(
        connect, L"GET", urlPath, nullptr,
        WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES, flags);
    if (request == nullptr) {
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        return "";
    }
    WinHttpAddRequestHeaders(request,
        L"User-Agent: VocaWin-Updater\r\n",
        static_cast<DWORD>(-1L), WINHTTP_ADDREQ_FLAG_ADD);
    if (!WinHttpSendRequest(request, WINHTTP_NO_ADDITIONAL_HEADERS, 0,
                            WINHTTP_NO_REQUEST_DATA, 0, 0, 0)) {
        WinHttpCloseHandle(request);
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        return "";
    }
    if (!WinHttpReceiveResponse(request, nullptr)) {
        WinHttpCloseHandle(request);
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        return "";
    }
    std::string body;
    char buf[4096];
    DWORD bytesRead = 0;
    while (WinHttpReadData(request, buf, sizeof(buf), &bytesRead) &&
           bytesRead > 0) {
        body.append(buf, bytesRead);
    }
    WinHttpCloseHandle(request);
    WinHttpCloseHandle(connect);
    WinHttpCloseHandle(session);
    return body;
}
#endif

}  // namespace

Updater::Version Updater::parseVersion(const std::string& tag) {
    std::string s = stripPrefix(tag, "v");
    s = stripPrefix(s, "V");
    Version v;
    std::replace_if(s.begin(), s.end(),
                    [](char c) {
                        return !std::isdigit(static_cast<unsigned char>(c)) &&
                               c != '.';
                    },
                    '.');
    std::vector<std::uint32_t> parts;
    std::istringstream is(s);
    std::string part;
    while (std::getline(is, part, '.')) {
        if (part.empty()) continue;
        try {
            parts.push_back(static_cast<std::uint32_t>(std::stoul(part)));
        } catch (...) {
            parts.push_back(0);
        }
    }
    if (parts.size() > 0) v.major = parts[0];
    if (parts.size() > 1) v.minor = parts[1];
    if (parts.size() > 2) v.patch = parts[2];
    return v;
}

bool Updater::isNewer(const std::string& current, const std::string& latest) {
    const Version a = parseVersion(current);
    const Version b = parseVersion(latest);
    if (a.major != b.major) return a.major < b.major;
    if (a.minor != b.minor) return a.minor < b.minor;
    return a.patch < b.patch;
}

std::string Updater::defaultChannel() {
    return "https://api.github.com/repos/vocawin/vocawin/releases/latest";
}

std::string Updater::latestTag(const std::string& channel) {
#if defined(_WIN32)
    const std::string url = channel.empty() ? defaultChannel() : channel;
    const std::string body = httpGet(url);
    if (body.empty()) return "";
    return extractTagName(body);
#else
    (void)channel;
    return "";
#endif
}

std::string Updater::latestDownloadUrl(const std::string& channel) {
#if defined(_WIN32)
    const std::string url = channel.empty() ? defaultChannel() : channel;
    const std::string body = httpGet(url);
    if (body.empty()) return "";
    return extractDownloadUrl(body);
#else
    (void)channel;
    return "";
#endif
}

}  // namespace vocawin
