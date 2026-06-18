cmake_minimum_required(VERSION 3.21)

project(VocaWin VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(VOCAWIN_BUILD_TESTS "Build unit tests" ON)
option(VOCAWIN_WITH_WHISPER "Build with whisper.cpp speech recognition" ON)

include(FetchContent)

set(WHISPER_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(WHISPER_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(WHISPER_BUILD_SERVER OFF CACHE BOOL "" FORCE)
set(GGML_OPENMP OFF CACHE BOOL "" FORCE)
set(GGML_NATIVE OFF CACHE BOOL "" FORCE)
set(GGML_CPU_ALL_VARIANTS OFF CACHE BOOL "" FORCE)
set(GGML_CUDA OFF CACHE BOOL "" FORCE)
set(GGML_METAL OFF CACHE BOOL "" FORCE)
set(GGML_VULKAN OFF CACHE BOOL "" FORCE)
set(GGML_OPENCL OFF CACHE BOOL "" FORCE)
set(GGML_RPC OFF CACHE BOOL "" FORCE)
set(GGML_SYCL OFF CACHE BOOL "" FORCE)
set(GGML_BLAS OFF CACHE BOOL "" FORCE)

if(VOCAWIN_WITH_WHISPER)
    FetchContent_Declare(
        whisper.cpp
        GIT_REPOSITORY https://github.com/ggerganov/whisper.cpp.git
        GIT_TAG        v1.8.7
        GIT_SHALLOW    TRUE
    )
    FetchContent_MakeAvailable(whisper.cpp)
endif()

add_library(vocawin_core STATIC
    src/app/AppController.cpp
    src/app/SingleInstance.cpp
    src/audio/AudioBuffer.cpp
    src/audio/SilenceDetector.cpp
    src/audio/SoundFeedback.cpp
    src/config/Settings.cpp
    src/config/SettingsStore.cpp
    src/input/ClipboardManager.cpp
    src/input/HotkeyManager.cpp
    src/input/TextInjector.cpp
    src/ui/TrayIcon.cpp
    src/util/Logger.cpp
)

target_include_directories(vocawin_core
    PUBLIC
        src
)

target_compile_definitions(vocawin_core PRIVATE
    UNICODE
    _UNICODE
)

if(WIN32)
    target_link_libraries(vocawin_core PRIVATE advapi32 user32 winmm)
endif()

if(VOCAWIN_WITH_WHISPER)
    target_link_libraries(vocawin_core PRIVATE whisper)
    target_include_directories(vocawin_core PUBLIC
        ${whisper.cpp_SOURCE_DIR}/include
        ${whisper.cpp_SOURCE_DIR}/ggml/include
    )
endif()

add_executable(vocawin
    src/main.cpp
)

target_link_libraries(vocawin PRIVATE vocawin_core)

if(VOCAWIN_BUILD_TESTS)
    include(CTest)
    enable_testing()
    add_subdirectory(tests)
endif()
