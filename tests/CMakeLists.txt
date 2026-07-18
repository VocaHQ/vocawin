set(VOCAWIN_TESTS
    test_single_instance
    test_settings_store
    test_app_controller
    test_logger
    test_tray_icon
    test_audio_buffer
    test_silence_detector
    test_clipboard_manager
    test_text_injector
    test_sound_feedback
    test_whisper_compile
    test_hotkey_manager
    test_hotkey_transcribe
    test_model_manager
    test_audio_capture
    test_whisper_engine
    test_mvp_pipeline
    test_settings_window
    test_autostart
    test_system_info
    test_gpu_detector
    test_notification
    test_onboarding
    test_overlay
    test_updater
)

foreach(t IN LISTS VOCAWIN_TESTS)
    add_executable(${t} ${t}.cpp)
    target_include_directories(${t} PRIVATE ${PROJECT_SOURCE_DIR}/src)
    target_link_libraries(${t} PRIVATE vocawin_core)
    if(WIN32)
        set_target_properties(${t} PROPERTIES
            WIN32_EXECUTABLE FALSE)
        if(_vocawin_windres AND EXISTS "${_vocawin_rc}")
            set(_test_rc_o
                "${CMAKE_CURRENT_BINARY_DIR}/${t}_manifest_res.o")
            add_custom_command(
                OUTPUT "${_test_rc_o}"
                DEPENDS "${_vocawin_rc}"
                    "${PROJECT_SOURCE_DIR}/resources/manifest/app.manifest"
                COMMAND ${_vocawin_windres} -i "${_vocawin_rc}" -O coff
                    -o "${_test_rc_o}"
                COMMENT "Compiling manifest for ${t}"
                VERBATIM)
            target_sources(${t} PRIVATE "${_test_rc_o}")
        endif()
        # The retry wrapper absorbs transient Defender scan locks that
        # cause ERROR_ACCESS_DENIED on newly-written exes.
        add_test(NAME ${t}
            COMMAND ${CMAKE_COMMAND} -E chdir
                $<TARGET_FILE_DIR:${t}>
                powershell -NoProfile -ExecutionPolicy Bypass -File
                "${PROJECT_SOURCE_DIR}/scripts/run_with_retry.ps1"
                ${t}.exe)
        # Headless: skip MessageBox / interactive UI in Onboarding and similar.
        set_tests_properties(${t} PROPERTIES
            ENVIRONMENT "VOCAWIN_HEADLESS=1"
            TIMEOUT 180)
    else()
        add_test(NAME ${t} COMMAND ${t})
        set_tests_properties(${t} PROPERTIES
            ENVIRONMENT "VOCAWIN_HEADLESS=1"
            TIMEOUT 180)
    endif()
endforeach()

# Integrated hotkey + real tiny.en inference needs a longer budget on CPU/SSE.
if(TARGET test_hotkey_transcribe)
    set_tests_properties(test_hotkey_transcribe PROPERTIES TIMEOUT 300)
endif()

if(WIN32)
    # Stage the MinGW runtime DLLs next to every test executable so the
    # Windows loader finds them without requiring PATH to be set. The
    # DLLs are searched in: (1) the compiler's directory, (2) a
    # MINGW_ROOT env var, (3) a CMake cache variable.
    set(_vocawin_mingw_dll_dir "")
    if(EXISTS "${CMAKE_CXX_COMPILER_DIRECTORY}/libgcc_s_seh-1.dll")
        set(_vocawin_mingw_dll_dir "${CMAKE_CXX_COMPILER_DIRECTORY}")
    elseif(EXISTS "$ENV{MINGW_ROOT}/bin/libgcc_s_seh-1.dll")
        set(_vocawin_mingw_dll_dir "$ENV{MINGW_ROOT}/bin")
    elseif(EXISTS "${VOCAWIN_MINGW_BIN}/libgcc_s_seh-1.dll")
        set(_vocawin_mingw_dll_dir "${VOCAWIN_MINGW_BIN}")
    endif()
    if(_vocawin_mingw_dll_dir)
        file(GLOB _vocawin_dlls
            "${_vocawin_mingw_dll_dir}/libgcc_s_seh-1.dll"
            "${_vocawin_mingw_dll_dir}/libstdc++-6.dll"
            "${_vocawin_mingw_dll_dir}/libwinpthread-1.dll")
        foreach(_dll ${_vocawin_dlls})
            foreach(_tgt ${VOCAWIN_TESTS})
                add_custom_command(TARGET ${_tgt} POST_BUILD
                    COMMAND ${CMAKE_COMMAND} -E copy_if_different
                        "${_dll}" "$<TARGET_FILE_DIR:${_tgt}>/")
            endforeach()
        endforeach()
    endif()
endif()
