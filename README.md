# Serial Reader

A lightweight command-line utility written in C for detecting serial devices and reading data from serial ports on Linux.

The application supports serial device discovery through `libsystemd` and configurable serial communication parameters, including baud rate, data bits, parity, and stop bits.

## Features

- **Serial device discovery** — list supported serial devices and their metadata.
- **Device filtering** — scan all supported devices or filter by device type.
- **Serial data reading** — read raw data from a selected serial port and write it directly to standard output.
- **Configurable communication parameters** — set baud rate, data bits, parity, and stop bits.
- **Non-blocking I/O** — read serial data without blocking indefinitely on individual read operations.
- **Graceful shutdown** — handle `Ctrl+C` and close the serial connection.
- **CMake build system** — build the application using CMake and a C17-compatible compiler.

## Requirements

The following tools and libraries are required to build the project:

- C compiler with C17 support, such as GCC or Clang
- CMake 3.10 or newer
- `pkg-config`
- `libsystemd` development files
- Linux environment with serial device support

### Install dependencies on Debian, Ubuntu, or Parrot OS

```bash
sudo apt update
sudo apt install build-essential cmake pkg-config libsystemd-dev
```

Verify the installed tools:

```bash
cc --version
cmake --version
pkg-config --modversion libsystemd
```

## Building

### 1. Clone the repository

```bash
git clone https://github.com/thekamik/serial-reader.git
cd serial-reader
```

### 2. Configure the project

Create a separate build directory and configure the project with CMake:

```bash
cmake -S . -B build
```

CMake will detect the compiler, locate `pkg-config`, and check for the required `libsystemd` development files.

### 3. Compile

```bash
cmake --build build --parallel
```

The resulting executable is named `serial-reader` and is placed in the build directory:

```text
build/serial-reader
```

### 4. Verify the build

Display the command-line help:

```bash
./build/serial-reader --help
```

## Usage

The application provides two operating modes:

1. **Scan mode** — discover supported serial devices.
2. **Read mode** — connect to a specific serial device and read incoming data.

### Command syntax

```text
serial-reader DEVICE [OPTIONS]
serial-reader --scan [--type TYPE]
```

Display the full help message:

```bash
./build/serial-reader --help
```

## Scanning for serial devices

To list all supported serial devices:

```bash
./build/serial-reader --scan
```

To list only devices whose names begin with `/dev/ttyACM`:

```bash
./build/serial-reader --scan --type ttyACM
```

To list only devices whose names begin with `/dev/ttyUSB`:

```bash
./build/serial-reader --scan --type ttyUSB
```

The output contains the following columns:

| Column | Description |
|---|---|
| `PORT` | Serial device path |
| `PRODUCENT` | Device manufacturer |
| `MODEL` | Device model |
| `SERIAL` | Device serial number |

For example, a device might be listed as `/dev/ttyACM0` or `/dev/ttyUSB0`.

The actual device path and available metadata depend on the connected hardware and the information exposed by the Linux device subsystem.

Missing metadata is displayed as `-`.

## Reading serial data

To read data from a serial device, pass its device path as the first positional argument.

For example:

```bash
./build/serial-reader /dev/ttyACM0
```

The application opens the device, configures the serial connection, and prints received bytes to standard output.

To stop reading, press `Ctrl+C`.

### Configuring serial parameters

The application supports the following options:

| Option | Description | Default |
|---|---|---|
| `--baud RATE` | Baud rate | `9600` |
| `--data BITS` | Number of data bits: `5`, `6`, `7`, or `8` | `8` |
| `--parity TYPE` | Parity: `none`, `even`, or `odd` | `none` |
| `--stop BITS` | Number of stop bits: `1` or `2` | `1` |

The default configuration is **9600 8N1**:

- Baud rate: 9600
- Data bits: 8
- Parity: none
- Stop bits: 1

Supported baud rates in the current implementation are:

- `9600`
- `19200`
- `38400`
- `57600`
- `115200`

The selected parameters must match the configuration of the connected device.

### Examples

Read using the default configuration:

```bash
./build/serial-reader /dev/ttyUSB0
```

Read at 115200 baud:

```bash
./build/serial-reader /dev/ttyUSB0 --baud 115200
```

Use 7 data bits, even parity, and 1 stop bit:

```bash
./build/serial-reader /dev/ttyUSB0 \
  --baud 9600 \
  --data 7 \
  --parity even \
  --stop 1
```

Use 8 data bits, no parity, and 2 stop bits:

```bash
./build/serial-reader /dev/ttyACM0 \
  --baud 19200 \
  --data 8 \
  --parity none \
  --stop 2
```

## Permissions

On Linux, access to serial devices is controlled by file permissions and system configuration.

If the application cannot open a device because of insufficient permissions, inspect its ownership and permissions:

```bash
ls -l /dev/ttyACM0
```

On Debian-based systems, serial devices are commonly assigned to the `dialout` group. If your device uses this group, add your user to it:

```bash
sudo usermod -aG dialout "$USER"
```

Log out and log back in for the new group membership to take effect.

Verify your group membership:

```bash
groups
```

Use the actual group assigned to your device if it differs from `dialout`.

Running the application with `sudo` may help diagnose a permissions issue, but granting the appropriate device permissions is preferable for regular use.

## How it works

### Device discovery

The application uses `libsystemd` and its `sd-device` API to enumerate devices and retrieve available metadata.

In scan mode, it filters device paths according to the selected type:

- `all` — `/dev/ttyACM*` and `/dev/ttyUSB*`
- `ttyACM` — `/dev/ttyACM*`
- `ttyUSB` — `/dev/ttyUSB*`

This means that scan mode does not currently attempt to discover every possible serial interface, such as `/dev/ttyS*`.

### Serial connection

In read mode, the application:

1. Opens the selected serial device.
2. Configures the terminal interface using `termios`.
3. Applies the requested baud rate, data bits, parity, and stop bits.
4. Waits for readable data using `select()`.
5. Reads available bytes and writes them to standard output.
6. Closes the connection when interrupted or when a read error occurs.

The serial device is opened in non-blocking mode.

### Data handling

Received bytes are written directly to standard output. The application does not currently implement a higher-level message protocol, JSON serialization, or automatic numeric conversion.

The received data should therefore be interpreted according to the protocol used by the connected device.

## Project structure

```text
serial-reader/
├── CMakeLists.txt
├── LICENSE
├── include/
│   ├── devices.h
│   ├── parser.h
│   └── serial.h
└── src/
    ├── devices.c
    ├── main.c
    ├── parser.c
    └── serial.c
```

### Source files

| File | Responsibility |
|---|---|
| `src/main.c` | Application entry point, operating modes, read loop, and signal handling |
| `src/parser.c` | Command-line argument parsing, validation, defaults, and help output |
| `src/serial.c` | Serial port configuration, opening, reading, writing, and closing |
| `src/devices.c` | Serial device enumeration and metadata retrieval using `libsystemd` |
| `include/parser.h` | Command-line parser declarations and configuration types |
| `include/serial.h` | Serial port interface and configuration declarations |
| `include/devices.h` | Device discovery interface and data structures |
| `CMakeLists.txt` | Build configuration and dependency declarations |

## Error handling

The application reports errors encountered during argument parsing, device discovery, serial configuration, and data reading.

Common issues include:

- **Missing serial device** — provide a device path when running in read mode.
- **Invalid option** — check the option spelling and its expected value.
- **Unsupported baud rate** — use one of the baud rates listed above.
- **Permission denied** — verify access permissions for the selected device.
- **Device not found** — check that the hardware is connected and that the device path exists.
- **Missing dependency** — install `libsystemd-dev`, `pkg-config`, and the required build tools.

For additional diagnostics, inspect the error message printed by the application and the associated system error.

## Development

The project uses C17 and CMake.

To rebuild after modifying the source code:

```bash
cmake --build build --parallel
```

To reconfigure the project:

```bash
cmake -S . -B build
```

To perform a clean rebuild:

```bash
cmake --build build --target clean
cmake --build build --parallel
```

The build directory contains generated build files and the compiled executable. It should generally remain separate from the source tree.

## Limitations

The current implementation has the following limitations:

- Device discovery is limited to `/dev/ttyACM*` and `/dev/ttyUSB*`.
- Only the baud rates explicitly supported by the implementation can be selected.
- Incoming data is emitted as raw bytes; message framing and protocol decoding are left to the caller.
- The application does not provide a graphical interface.
- The application does not currently expose a dedicated configuration file; serial settings are supplied through command-line arguments.

## License

This project is distributed under the MIT License. See [LICENSE](LICENSE) for the full license text.
