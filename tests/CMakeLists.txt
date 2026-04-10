add_executable(vocawin_tests
    test_single_instance.cpp
    test_settings_store.cpp
)

target_include_directories(vocawin_tests PRIVATE
    ${PROJECT_SOURCE_DIR}/src
)

target_link_libraries(vocawin_tests PRIVATE
    vocawin_core
)

add_test(NAME vocawin_tests COMMAND vocawin_tests)
