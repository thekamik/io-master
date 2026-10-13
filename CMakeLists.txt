cmake_minimum_required(VERSION 3.10)  # Specify CMake version
project(io-master VERSION 0.2 LANGUAGES C)  # Project name and version

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

find_package(PkgConfig REQUIRED)
pkg_check_modules(SYSTEMD REQUIRED libsystemd)
pkg_check_modules(JANSSON REQUIRED jansson)
pkg_check_modules(CURL REQUIRED libcurl)
pkg_check_modules(SECRET REQUIRED libsecret-1)

# Add executable
add_executable(main
    ${PROJECT_SOURCE_DIR}/src/main.c
    ${PROJECT_SOURCE_DIR}/src/cli/parser.c
    ${PROJECT_SOURCE_DIR}/src/cli/commands.c
    ${PROJECT_SOURCE_DIR}/src/cli/ai_config.c
    ${PROJECT_SOURCE_DIR}/src/project/project.c
    ${PROJECT_SOURCE_DIR}/src/storage/config_dir.c
    ${PROJECT_SOURCE_DIR}/src/storage/json_file.c
    ${PROJECT_SOURCE_DIR}/src/protocol/protocol.c
    ${PROJECT_SOURCE_DIR}/src/protocol/uart/uart.c
    ${PROJECT_SOURCE_DIR}/src/protocol/uart/uart_parser.c
    ${PROJECT_SOURCE_DIR}/src/protocol/uart/serial.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/bluetooth.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/bluetooth_parser.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/bluez.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/adapter.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/device.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/connection.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/agent.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/scan_table.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/sniffer.c
    ${PROJECT_SOURCE_DIR}/src/protocol/ai/ai_protocol.c
    ${PROJECT_SOURCE_DIR}/src/protocol/ai/ai_parser.c
    ${PROJECT_SOURCE_DIR}/src/ai/ai.c
    ${PROJECT_SOURCE_DIR}/src/ai/ai_settings.c
    ${PROJECT_SOURCE_DIR}/src/ai/chat_store.c
    ${PROJECT_SOURCE_DIR}/src/ai/anthropic.c
    ${PROJECT_SOURCE_DIR}/src/ai/gemini.c
    ${PROJECT_SOURCE_DIR}/src/ai/http.c
    ${PROJECT_SOURCE_DIR}/src/ai/keyring.c
    ${PROJECT_SOURCE_DIR}/src/discovery/devices.c
)

target_compile_features(main PRIVATE c_std_17)

target_compile_definitions(main PRIVATE
    _DEFAULT_SOURCE
    _POSIX_C_SOURCE=200809L
)

set_target_properties(main PROPERTIES
    OUTPUT_NAME "io-master"
)

# Include directories
target_include_directories(main PRIVATE
    ${PROJECT_SOURCE_DIR}/include
    ${SYSTEMD_INCLUDE_DIRS}
    ${JANSSON_INCLUDE_DIRS}
    ${CURL_INCLUDE_DIRS}
    ${SECRET_INCLUDE_DIRS}
)

target_link_libraries(main PRIVATE
    ${SYSTEMD_LIBRARIES}
    ${JANSSON_LIBRARIES}
    ${CURL_LIBRARIES}
    ${SECRET_LIBRARIES}
)

target_compile_options(main PRIVATE
    ${SYSTEMD_CFLAGS_OTHER}
    ${JANSSON_CFLAGS_OTHER}
    ${CURL_CFLAGS_OTHER}
    ${SECRET_CFLAGS_OTHER}
)

# Tests
#
# Run them with:  ctest --test-dir build --output-on-failure
#
# The tests are scripts in tests/ that drive the built executable.
# They need python3; see the README for the packages each test uses.
enable_testing()

# End-to-end test of the Bluetooth shell against a stand-in for BlueZ.
# It runs on a private D-Bus bus and never touches real devices.
add_test(
    NAME bluetooth_shell
    COMMAND python3
        ${PROJECT_SOURCE_DIR}/tests/bluetooth/test_bluetooth_shell.py
        $<TARGET_FILE:main>
)

# End-to-end test of the AI shell against a stand-in HTTPS server and
# a stand-in keyring. It needs no API key and never touches the real
# keyring of the user.
add_test(
    NAME ai_shell
    COMMAND python3
        ${PROJECT_SOURCE_DIR}/tests/ai/test_ai_shell.py
        $<TARGET_FILE:main>
)
