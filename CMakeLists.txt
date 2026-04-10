cmake_minimum_required(VERSION 3.21)

project(VocaWin VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(VOCAWIN_BUILD_TESTS "Build unit tests" ON)

add_library(vocawin_core STATIC
    src/app/AppController.cpp
    src/app/SingleInstance.cpp
    src/config/Settings.cpp
    src/config/SettingsStore.cpp
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
    target_link_libraries(vocawin_core PRIVATE advapi32)
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
