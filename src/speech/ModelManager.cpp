#include "speech/ModelManager.h"

#include <algorithm>
#include <fstream>
#include <system_error>
#include <vector>

#if defined(_WIN32)
#include <windows.h>
#include <winhttp.h>
#pragma comment(lib, "winhttp.lib")
#endif

namespace vocawin {

namespace {

constexpr std::size_t kMB = 1024ULL * 1024;

const std::vector<ModelManager::ModelInfo>& catalog() {
    static const std::vector<ModelManager::ModelInfo> kModels = {
        {"tiny.en",   "Tiny (English, 39M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
         75 * kMB,    273 * kMB},
        {"tiny",      "Tiny (Multilingual, 39M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
         75 * kMB,    273 * kMB},
        {"base.en",   "Base (English, 74M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
         142 * kMB,   388 * kMB},
        {"base",      "Base (Multilingual, 74M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
         142 * kMB,   388 * kMB},
        {"small.en",  "Small (English, 244M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
         466 * kMB,   852 * kMB},
        {"small",     "Small (Multilingual, 244M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
         466 * kMB,   852 * kMB},
        {"medium.en", "Medium (English, 769M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
         1500 * kMB,  2100 * kMB},
        {"medium",    "Medium (Multilingual, 769M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
         1500 * kMB,  2100 * kMB},
        {"large-v3",         "Large v3 (Multilingual, 1550M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
         2900 * kMB,  3900 * kMB},
        {"large-v3-turbo",   "Large v3 Turbo (Multilingual, 809M params)",
         "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
         809 * kMB,   1200 * kMB},
    };
    return kModels;
}

const ModelManager::ModelInfo* findById(const std::string& id) {
    const auto& m = catalog();
    auto it = std::find_if(m.begin(), m.end(),
                           [&id](const ModelManager::ModelInfo& mi) {
                               return mi.id == id;
                           });
    return it == m.end() ? nullptr : &(*it);
}

bool isFileUrl(const std::string& url) {
    constexpr const char* kFilePrefix = "file:///";
    return url.compare(0, 8, kFilePrefix) == 0;
}

bool isHttpUrl(const std::string& url) {
    return url.compare(0, 8, "https://") == 0 ||
           url.compare(0, 7, "http://") == 0;
}

// Write to dst via a sibling .part file, then rename. Avoids half-written
// models if the process dies mid-download.
bool writeAtomic(const std::filesystem::path& dst,
                 const std::function<bool(std::ofstream&,
                                          ModelManager::ProgressCallback)>& writer,
                 ModelManager::ProgressCallback onProgress) {
    std::error_code ec;
    std::filesystem::create_directories(dst.parent_path(), ec);
    if (ec) {
        return false;
    }
    const auto part = dst.string() + ".part";
    {
        std::ofstream out(part, std::ios::binary | std::ios::trunc);
        if (!out.good()) {
            return false;
        }
        if (!writer(out, onProgress)) {
            out.close();
            std::filesystem::remove(part, ec);
            return false;
        }
        out.flush();
        if (!out.good()) {
            out.close();
            std::filesystem::remove(part, ec);
            return false;
        }
    }
    std::filesystem::remove(dst, ec);
    std::filesystem::rename(part, dst, ec);
    if (ec) {
        std::filesystem::remove(part, ec);
        return false;
    }
    if (onProgress) {
        onProgress(1.0f);
    }
    return true;
}

bool downloadFromFileUrl(const std::string& url,
                         const std::filesystem::path& dst,
                         ModelManager::ProgressCallback onProgress) {
    std::filesystem::path src = url.substr(8);  // after "file:///"
    return writeAtomic(
        dst,
        [&](std::ofstream& out, ModelManager::ProgressCallback progress) {
            std::ifstream in(src, std::ios::binary);
            if (!in.good()) {
                return false;
            }
            constexpr std::size_t kBuf = 64 * 1024;
            char buf[kBuf];
            std::uint64_t total = 0;
            in.seekg(0, std::ios::end);
            const std::streamoff len = in.tellg();
            in.seekg(0, std::ios::beg);
            while (in) {
                in.read(buf, static_cast<std::streamsize>(kBuf));
                const std::streamsize n = in.gcount();
                if (n <= 0) {
                    break;
                }
                out.write(buf, n);
                if (!out.good()) {
                    return false;
                }
                total += static_cast<std::uint64_t>(n);
                if (progress && len > 0) {
                    progress(static_cast<float>(
                        static_cast<double>(total) /
                        static_cast<double>(len)));
                }
            }
            return true;
        },
        onProgress);
}

#if defined(_WIN32)

std::wstring utf8ToWide(const std::string& s) {
    if (s.empty()) {
        return {};
    }
    const int needed = MultiByteToWideChar(
        CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0);
    if (needed <= 0) {
        return std::wstring(s.begin(), s.end());
    }
    std::wstring out(static_cast<std::size_t>(needed), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()),
                        out.data(), needed);
    return out;
}

bool downloadFromHttpUrl(const std::string& url,
                         const std::filesystem::path& dst,
                         std::uint64_t expectedBytes,
                         ModelManager::ProgressCallback onProgress) {
    const std::wstring wurl = utf8ToWide(url);

    HINTERNET session = WinHttpOpen(
        L"VocaWin/0.1",
        WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
        WINHTTP_NO_PROXY_NAME,
        WINHTTP_NO_PROXY_BYPASS,
        0);
    if (session == nullptr) {
        return false;
    }

    // Follow HTTPS redirects (HuggingFace resolve URLs use them).
    DWORD redirectPolicy = WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS;
    WinHttpSetOption(session, WINHTTP_OPTION_REDIRECT_POLICY,
                     &redirectPolicy, sizeof(redirectPolicy));

    URL_COMPONENTS uc{};
    uc.dwStructSize = sizeof(uc);
    wchar_t hostName[256] = {};
    wchar_t urlPath[4096] = {};
    uc.lpszHostName = hostName;
    uc.dwHostNameLength = 256;
    uc.lpszUrlPath = urlPath;
    uc.dwUrlPathLength = 4096;
    if (!WinHttpCrackUrl(wurl.c_str(), static_cast<DWORD>(wurl.size()), 0, &uc)) {
        WinHttpCloseHandle(session);
        return false;
    }

    HINTERNET connect = WinHttpConnect(session, hostName, uc.nPort, 0);
    if (connect == nullptr) {
        WinHttpCloseHandle(session);
        return false;
    }

    const DWORD flags = (uc.nScheme == INTERNET_SCHEME_HTTPS)
                            ? WINHTTP_FLAG_SECURE
                            : 0;
    HINTERNET request = WinHttpOpenRequest(
        connect, L"GET", urlPath, nullptr,
        WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES, flags);
    if (request == nullptr) {
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        return false;
    }

    WinHttpAddRequestHeaders(
        request,
        L"User-Agent: VocaWin/0.1\r\n"
        L"Accept: */*\r\n",
        static_cast<DWORD>(-1L),
        WINHTTP_ADDREQ_FLAG_ADD);

    // Generous timeouts for multi-hundred-MB model files.
    DWORD timeoutMs = 600000;  // 10 minutes
    WinHttpSetOption(request, WINHTTP_OPTION_RECEIVE_TIMEOUT,
                     &timeoutMs, sizeof(timeoutMs));
    WinHttpSetOption(request, WINHTTP_OPTION_SEND_TIMEOUT,
                     &timeoutMs, sizeof(timeoutMs));

    if (!WinHttpSendRequest(request, WINHTTP_NO_ADDITIONAL_HEADERS, 0,
                            WINHTTP_NO_REQUEST_DATA, 0, 0, 0) ||
        !WinHttpReceiveResponse(request, nullptr)) {
        WinHttpCloseHandle(request);
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        return false;
    }

    DWORD statusCode = 0;
    DWORD statusSize = sizeof(statusCode);
    if (!WinHttpQueryHeaders(request,
                             WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                             WINHTTP_HEADER_NAME_BY_INDEX, &statusCode,
                             &statusSize, WINHTTP_NO_HEADER_INDEX) ||
        statusCode < 200 || statusCode >= 300) {
        WinHttpCloseHandle(request);
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        return false;
    }

    std::uint64_t contentLength = 0;
    {
        wchar_t lenBuf[64] = {};
        DWORD lenSize = sizeof(lenBuf);
        if (WinHttpQueryHeaders(request, WINHTTP_QUERY_CONTENT_LENGTH,
                                WINHTTP_HEADER_NAME_BY_INDEX, lenBuf, &lenSize,
                                WINHTTP_NO_HEADER_INDEX)) {
            contentLength = static_cast<std::uint64_t>(_wcstoui64(lenBuf, nullptr, 10));
        }
    }
    if (contentLength == 0 && expectedBytes > 0) {
        contentLength = expectedBytes;
    }

    const bool ok = writeAtomic(
        dst,
        [&](std::ofstream& out, ModelManager::ProgressCallback progress) {
            constexpr DWORD kBuf = 64 * 1024;
            std::vector<char> buf(kBuf);
            std::uint64_t total = 0;
            DWORD bytesRead = 0;
            while (WinHttpReadData(request, buf.data(), kBuf, &bytesRead) &&
                   bytesRead > 0) {
                out.write(buf.data(), static_cast<std::streamsize>(bytesRead));
                if (!out.good()) {
                    return false;
                }
                total += bytesRead;
                if (progress) {
                    if (contentLength > 0) {
                        const float p = static_cast<float>(
                            static_cast<double>(total) /
                            static_cast<double>(contentLength));
                        progress(p > 1.0f ? 1.0f : p);
                    } else {
                        // Unknown length: asymptotic progress so UI moves.
                        progress(1.0f - 1.0f / (1.0f + static_cast<float>(total) /
                                                           (1024.0f * 1024.0f)));
                    }
                }
                bytesRead = 0;
            }
            // Require at least a few KB so empty error bodies don't look like success.
            return total >= 1024;
        },
        onProgress);

    WinHttpCloseHandle(request);
    WinHttpCloseHandle(connect);
    WinHttpCloseHandle(session);
    return ok;
}

#else  // !_WIN32

bool downloadFromHttpUrl(const std::string&,
                         const std::filesystem::path&,
                         std::uint64_t,
                         ModelManager::ProgressCallback) {
    return false;
}

#endif

}  // namespace

ModelManager::ModelManager(std::filesystem::path modelsDir)
    : modelsDir_(std::move(modelsDir)) {}

std::vector<ModelManager::ModelInfo>
ModelManager::getAvailableModels() {
    return catalog();
}

std::vector<ModelManager::ModelInfo>
ModelManager::getLocalModels() const {
    std::vector<ModelInfo> out;
    for (const auto& m : catalog()) {
        const auto p = getModelPath(m.id);
        if (std::filesystem::exists(p)) {
            out.push_back(m);
        }
    }
    return out;
}

bool ModelManager::isModelDownloaded(const std::string& modelId) const {
    return std::filesystem::exists(getModelPath(modelId));
}

std::filesystem::path
ModelManager::getModelPath(const std::string& modelId) const {
    return modelsDir_ / ("ggml-" + modelId + ".bin");
}

bool ModelManager::downloadModel(const std::string& modelId,
                                 const std::string& url,
                                 ProgressCallback onProgress) {
    const auto* meta = findById(modelId);
    if (meta == nullptr) {
        return false;
    }
    std::string effectiveUrl = url;
    if (effectiveUrl.empty()) {
        effectiveUrl = meta->url;
    }
    if (effectiveUrl.empty()) {
        return false;
    }

    const auto dst = getModelPath(modelId);

    if (isFileUrl(effectiveUrl)) {
        return downloadFromFileUrl(effectiveUrl, dst, onProgress);
    }
    if (isHttpUrl(effectiveUrl)) {
        return downloadFromHttpUrl(effectiveUrl, dst, meta->fileSizeBytes,
                                   onProgress);
    }
    // Unsupported scheme — never silent success.
    return false;
}

ModelManager::ModelInfo
ModelManager::recommendModel(std::size_t ramBytes, std::size_t vramBytes,
                              bool hasGpu) {
    auto pick = [](const std::string& id) -> ModelInfo {
        const auto* m = findById(id);
        return m ? *m : ModelInfo{};
    };
    if (hasGpu) {
        if (vramBytes >= 8ULL * 1024 * kMB) return pick("medium.en");
        if (vramBytes >= 4ULL * 1024 * kMB) return pick("small.en");
        if (vramBytes >= 2ULL * 1024 * kMB) return pick("base.en");
        return pick("base.en");
    }
    if (ramBytes >= 32ULL * 1024 * kMB) return pick("small.en");
    if (ramBytes >= 16ULL * 1024 * kMB) return pick("base.en");
    if (ramBytes >= 8ULL * 1024 * kMB)  return pick("tiny.en");
    return pick("tiny.en");
}

}  // namespace vocawin
