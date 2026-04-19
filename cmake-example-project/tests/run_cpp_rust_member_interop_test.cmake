if(NOT DEFINED PROJECT_BINARY_DIR_PATH)
    message(FATAL_ERROR "PROJECT_BINARY_DIR_PATH is required")
endif()
if(NOT DEFINED TRANSPILE_TARGET_NAME)
    message(FATAL_ERROR "TRANSPILE_TARGET_NAME is required")
endif()
if(NOT DEFINED BASH_EXECUTABLE_PATH)
    message(FATAL_ERROR "BASH_EXECUTABLE_PATH is required")
endif()
if(NOT DEFINED INTEROP_SCRIPT_PATH)
    message(FATAL_ERROR "INTEROP_SCRIPT_PATH is required")
endif()
if(NOT DEFINED RUSTYCPP_DIR_PATH)
    message(FATAL_ERROR "RUSTYCPP_DIR_PATH is required")
endif()
if(NOT DEFINED INTEROP_WORK_DIR_PATH)
    message(FATAL_ERROR "INTEROP_WORK_DIR_PATH is required")
endif()
if(NOT DEFINED TRANSPILED_CPPM_PATH)
    message(FATAL_ERROR "TRANSPILED_CPPM_PATH is required")
endif()

execute_process(
    COMMAND
        "${CMAKE_COMMAND}"
        --build
        "${PROJECT_BINARY_DIR_PATH}"
        --target
        "${TRANSPILE_TARGET_NAME}"
    RESULT_VARIABLE BUILD_RESULT
)
if(NOT BUILD_RESULT EQUAL 0)
    message(FATAL_ERROR "Failed to build target ${TRANSPILE_TARGET_NAME} (result=${BUILD_RESULT})")
endif()

execute_process(
    COMMAND
        "${BASH_EXECUTABLE_PATH}"
        "${INTEROP_SCRIPT_PATH}"
        --rustycpp-dir
        "${RUSTYCPP_DIR_PATH}"
        --work-dir
        "${INTEROP_WORK_DIR_PATH}"
        --transpiled-cppm
        "${TRANSPILED_CPPM_PATH}"
    RESULT_VARIABLE RUN_RESULT
)
if(NOT RUN_RESULT EQUAL 0)
    message(FATAL_ERROR "Interop smoke test failed (result=${RUN_RESULT})")
endif()
