add_executable(test_single_instance
    test_single_instance.cpp
)

target_include_directories(test_single_instance PRIVATE
    ${PROJECT_SOURCE_DIR}/src
)

target_link_libraries(test_single_instance PRIVATE
    vocawin_core
)

add_test(NAME test_single_instance COMMAND test_single_instance)

add_executable(test_settings_store
    test_settings_store.cpp
)

target_include_directories(test_settings_store PRIVATE
    ${PROJECT_SOURCE_DIR}/src
)

target_link_libraries(test_settings_store PRIVATE
    vocawin_core
)

add_test(NAME test_settings_store COMMAND test_settings_store)

add_executable(test_app_controller
    test_app_controller.cpp
)

target_include_directories(test_app_controller PRIVATE
    ${PROJECT_SOURCE_DIR}/src
)

target_link_libraries(test_app_controller PRIVATE
    vocawin_core
)

add_test(NAME test_app_controller COMMAND test_app_controller)

add_executable(test_logger
    test_logger.cpp
)

target_include_directories(test_logger PRIVATE
    ${PROJECT_SOURCE_DIR}/src
)

target_link_libraries(test_logger PRIVATE
    vocawin_core
)

add_test(NAME test_logger COMMAND test_logger)

add_executable(test_tray_icon
    test_tray_icon.cpp
)

target_include_directories(test_tray_icon PRIVATE
    ${PROJECT_SOURCE_DIR}/src
)

target_link_libraries(test_tray_icon PRIVATE
    vocawin_core
)

add_test(NAME test_tray_icon COMMAND test_tray_icon)
