cmake_minimum_required(VERSION 3.10)  # Specify CMake version
project(io-master VERSION 1.1 LANGUAGES C)  # Project name and version

set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

find_package(PkgConfig REQUIRED)
pkg_check_modules(SYSTEMD REQUIRED libsystemd)
pkg_check_modules(JANSSON REQUIRED jansson)

# Add executable
add_executable(main
    ${PROJECT_SOURCE_DIR}/src/main.c
    ${PROJECT_SOURCE_DIR}/src/cli/parser.c
    ${PROJECT_SOURCE_DIR}/src/cli/commands.c
    ${PROJECT_SOURCE_DIR}/src/project/project.c
    ${PROJECT_SOURCE_DIR}/src/storage/config_dir.c
    ${PROJECT_SOURCE_DIR}/src/storage/json_file.c
    ${PROJECT_SOURCE_DIR}/src/protocol/protocol.c
    ${PROJECT_SOURCE_DIR}/src/protocol/uart/uart.c
    ${PROJECT_SOURCE_DIR}/src/protocol/uart/uart_parser.c
    ${PROJECT_SOURCE_DIR}/src/protocol/uart/serial.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/bluetooth.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/bluetooth_parser.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/adapter.c
    ${PROJECT_SOURCE_DIR}/src/protocol/bluetooth/sniffer.c
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
)

target_link_libraries(main PRIVATE
    ${SYSTEMD_LIBRARIES}
    ${JANSSON_LIBRARIES}
)

target_compile_options(main PRIVATE
    ${SYSTEMD_CFLAGS_OTHER}
    ${JANSSON_CFLAGS_OTHER}
)
