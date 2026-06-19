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
    src/audio/AudioCapture.cpp
    src/audio/SilenceDetector.cpp
    src/audio/SoundFeedback.cpp
    src/config/Settings.cpp
    src/config/SettingsStore.cpp
    src/input/ClipboardManager.cpp
    src/input/HotkeyManager.cpp
    src/input/TextInjector.cpp
    src/platform/Autostart.cpp
    src/platform/GpuDetector.cpp
    src/platform/Notification.cpp
    src/platform/SystemInfo.cpp
    src/speech/ModelManager.cpp
    src/speech/WhisperEngine.cpp
    src/ui/OnboardingWindow.cpp
    src/ui/OverlayWindow.cpp
    src/ui/SettingsWindow.cpp
    src/ui/TrayIcon.cpp
    src/updater/Updater.cpp
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
    target_link_libraries(vocawin_core PRIVATE advapi32 user32 winmm ole32 mmdevapi shell32 comctl32 winhttp)
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

if(WIN32 AND MINGW)
    # Compile the application manifest into a COFF object via windres
    # and link it into vocawin.exe. Without an embedded asInvoker
    # manifest, Windows falls back to SmartScreen heuristics and
    # shows a UAC-style "unknown publisher" prompt when the exe is
    # launched via ShellExecuteEx (Start-Process, taskbar pin,
    # installer). ld.exe does not support the MSVC --manifest flag,
    # so we go through windres.
    find_program(_vocawin_windres windres
        HINTS ${CMAKE_CXX_COMPILER_DIRECTORY}
              $ENV{MINGW_ROOT}/bin
        DOC   "MinGW windres (manifest resource compiler)")
    set(_vocawin_rc
        "${PROJECT_SOURCE_DIR}/resources/manifest/manifest.rc"
        CACHE FILEPATH "Vocawin manifest .rc file")
    set(_vocawin_windres "${_vocawin_windres}"
        CACHE FILEPATH "Vocawin windres binary")
    set(_vocawin_rc_o
        "${CMAKE_CURRENT_BINARY_DIR}/vocawin_manifest_res.o")
    if(_vocawin_windres AND EXISTS "${_vocawin_rc}")
        add_custom_command(
            OUTPUT "${_vocawin_rc_o}"
            DEPENDS "${_vocawin_rc}"
                "${PROJECT_SOURCE_DIR}/resources/manifest/app.manifest"
            COMMAND ${_vocawin_windres} -i "${_vocawin_rc}" -O coff
                -o "${_vocawin_rc_o}"
            COMMENT "Compiling application manifest (windres)"
            VERBATIM)
        target_sources(vocawin PRIVATE "${_vocawin_rc_o}")
    endif()
endif()

if(VOCAWIN_BUILD_TESTS)
    include(CTest)
    enable_testing()
    add_subdirectory(tests)
endif()

if(WIN32)
    # Copy runtime resources (icons, sounds) next to vocawin.exe so
    # TrayIcon and SoundFeedback can load them at runtime without
    # relying on a known install path.
    set(_vocawin_res_dir "${PROJECT_SOURCE_DIR}/resources")
    if(EXISTS "${_vocawin_res_dir}/icons/tray-idle.ico")
        file(GLOB _vocawin_icon_files
            "${_vocawin_res_dir}/icons/tray-*.ico"
            "${_vocawin_res_dir}/icons/vocawin.ico")
        foreach(_icon ${_vocawin_icon_files})
            add_custom_command(TARGET vocawin POST_BUILD
                COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "${_icon}" "$<TARGET_FILE_DIR:vocawin>/"
                COMMENT "Staging ${_icon}")
        endforeach()
    endif()
    if(EXISTS "${_vocawin_res_dir}/sounds/start.wav")
        file(GLOB _vocawin_sound_files "${_vocawin_res_dir}/sounds/*.wav")
        foreach(_snd ${_vocawin_sound_files})
            add_custom_command(TARGET vocawin POST_BUILD
                COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "${_snd}" "$<TARGET_FILE_DIR:vocawin>/"
                COMMENT "Staging ${_snd}")
        endforeach()
    endif()
endif()

# CPack packaging (MSI via WiX). The .wxs definition lives in
# installer/VocaWin.wxs; CPack invokes WiX v4 at `cpack -G WIX`.
set(CPACK_PACKAGE_NAME "VocaWin")
set(CPACK_PACKAGE_VENDOR "VocaWin Contributors")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY
    "100% offline voice-to-text for Windows.")
set(CPACK_PACKAGE_VERSION_MAJOR 0)
set(CPACK_PACKAGE_VERSION_MINOR 1)
set(CPACK_PACKAGE_VERSION_PATCH 0)
set(CPACK_GENERATOR "WIX")
set(CPACK_WIX_TEMPLATE "${PROJECT_SOURCE_DIR}/installer/VocaWin.wxs")
set(CPACK_PACKAGE_FILE_NAME "VocaWin-0.1.0-win64")
set(CPACK_INSTALL_DIRECTORY "VocaWin")
install(TARGETS vocawin RUNTIME DESTINATION .)
install(DIRECTORY "${PROJECT_SOURCE_DIR}/resources/icons"
        DESTINATION .)
install(DIRECTORY "${PROJECT_SOURCE_DIR}/resources/sounds"
        DESTINATION .)
include(CPack)
