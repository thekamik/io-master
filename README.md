# IO Master

**An evolving workflow integration tool for IoT and OT device research.**

IO Master is an open-source project aimed at making hardware and protocol research more efficient by bringing device discovery, communication, debugging tools, and investigation results into a more coherent workflow.

The project starts with serial device discovery and UART communication on Linux. Its long-term goal is to help researchers coordinate existing tools, manage reusable configurations, and correlate observations collected through different interfaces — without reinventing specialized tools that already do their jobs well.

**Project status:** Early development / MVP.

> IO Master is a work in progress. The current implementation focuses on serial device discovery and reading data from serial ports. Broader workflow integration features are planned and are not yet implemented.

## Motivation

Investigating an IoT or OT device often requires several tools and interfaces. A researcher might need to discover a USB-to-UART adapter, establish a serial connection, observe device logs, communicate with a service over Modbus, and inspect debugging activity through JTAG.

These activities are often performed in separate applications, each with its own configuration, command-line options, output format, and workflow.

IO Master aims to reduce this fragmentation by providing a common place to configure and coordinate investigation tasks, invoke existing tools, and organize their results.

The goal is **not to replace specialized tools or implement every analysis technique in one application**. Instead, IO Master should help connect the steps of an investigation and make their combined results easier to understand.

For example, a future workflow could involve:

1. Discovering available serial interfaces.
2. Connecting to a device through UART and capturing its output.
3. Starting a Modbus observation or communication task.
4. Launching or controlling an existing JTAG debugging tool.
5. Collecting outputs and recording events in a common investigation session.
6. Reviewing observations from different interfaces together to identify potentially related activity.

Such a workflow could help researchers investigate a device without repeatedly switching between unrelated tools and manually reconstructing what happened.

## Current features

The current MVP provides:

- **Serial device discovery** — enumerate supported serial devices and retrieve available device metadata.
- **Device filtering** — scan supported devices or filter by device type.
- **UART data reading** — open a selected serial port and print received bytes to standard output.
- **Serial communication configuration** — set baud rate, data bits, parity, and stop bits through command-line options.
- **Non-blocking serial I/O** — read available data without waiting indefinitely on an individual read operation.
- **Graceful shutdown** — handle `Ctrl+C` and close the serial connection.
- **Linux-native device discovery** — use `libsystemd` and its `sd-device` API to enumerate supported devices.

These features form the foundation for the broader workflow integration planned for the project.

## Design goals

### 1. Reusable configuration

Repeatedly supplying the same command-line options is inconvenient during long-running research projects.

IO Master is intended to support persistent configuration so that researchers can define their preferred settings once and reuse them across multiple invocations.

Potential configuration capabilities include:

- Persistent default serial settings.
- Named configurations or profiles for different devices and projects.
- Per-device communication parameters.
- Reusable settings for integrated tools and investigation tasks.
- Command-line overrides when a one-off change is needed.

The intended workflow is to configure an investigation once and subsequently invoke common operations with short, predictable commands.

**Status:** Planned. The current implementation uses command-line options and does not yet provide a dedicated configuration file or profile system.

### 2. Integration with existing tools

IO Master should coordinate specialized tools rather than reimplement their functionality.

For example, a future version could integrate with:

- Serial terminal and capture tools for UART communication.
- OpenOCD and compatible tooling for JTAG/SWD debugging.
- Modbus tools for observing and interacting with industrial communication interfaces.
- Other command-line utilities useful in IoT and OT security research.

Integration may involve launching processes, managing their arguments and configuration, collecting output, and controlling their lifecycle.

The exact integrations and control mechanisms will depend on the capabilities and interfaces exposed by each tool.

### 3. Investigation sessions

A research session should represent a collection of related activities performed while investigating a device.

A future session could contain:

- Device and interface information.
- Active communication or debugging tasks.
- Configuration profiles used by each task.
- Captured output and result-file locations.
- Start and stop times, errors, and operation status.
- Notes and events recorded during the investigation.

This would make it easier to preserve the context of an investigation and return to it later.

### 4. Cross-interface event correlation

Different interfaces can reveal different aspects of the same device's behavior. IO Master aims to make these observations easier to compare.

For example, a researcher could compare UART output with Modbus activity and JTAG-related debugging events on a common timeline.

A future event model could record timestamps, event sources, operation types, statuses, and links to captured data.

A shared timeline would help identify temporal relationships, but temporal proximity alone does not establish causation. Accurate correlation may also require accounting for clock differences, buffering, and tool-specific delays.

### 5. Modular architecture

IO Master is intended to grow through separate modules or adapters for different interfaces and external tools.

This approach should allow new integrations to be added without turning the project into a monolithic application or duplicating the functionality of established tools.

### 6. Complementary tools, not replacements

IO Master is not intended to replace every utility used during device research.

For example, firmware analysis can remain the responsibility of established tools such as Binwalk. IO Master may eventually help organize or invoke such tools as part of a broader workflow, but reproducing their specialized analysis engines is not a primary goal.

## Requirements

The current implementation requires:

- Linux with serial device support.
- A C compiler with C17 support, such as GCC or Clang.
- CMake 3.10 or newer.
- `pkg-config`.
- The `libsystemd` development package.

### Install dependencies on Debian, Ubuntu, or Parrot OS

```bash
sudo apt update
sudo apt install build-essential cmake pkg-config libsystemd-dev
```

## Building

### 1. Clone the repository

```bash
git clone https://github.com/thekamik/io-master.git
cd io-master
```

### 2. Configure the build

```bash
cmake -S . -B build
```

### 3. Compile

```bash
cmake --build build --parallel
```

The current build produces the `serial-reader` executable in the build directory:

```bash
./build/serial-reader --help
```

> The project has been renamed to IO Master, but the executable and command syntax may still use the original `serial-reader` name while the implementation is being updated.

## Current usage

The current implementation supports two main operations:

- Scanning for supported serial devices.
- Reading data from a selected serial device.

### Discover serial devices

List supported devices:

```bash
./build/serial-reader --scan
```

Filter for USB CDC ACM devices:

```bash
./build/serial-reader --scan --type ttyACM
```

Filter for USB serial devices:

```bash
./build/serial-reader --scan --type ttyUSB
```

The output may include:

| Field | Description |
|---|---|
| `PORT` | Serial device path |
| `PRODUCENT` | Device manufacturer, when available |
| `MODEL` | Device model, when available |
| `SERIAL` | Device serial number, when available |

Typical device paths include `/dev/ttyACM0` and `/dev/ttyUSB0`. The actual paths and metadata depend on the connected hardware and the information exposed by Linux.

### Read serial data

Read from a selected device using the default serial settings:

```bash
./build/serial-reader /dev/ttyUSB0
```

Read at 115200 baud:

```bash
./build/serial-reader /dev/ttyUSB0 --baud 115200
```

Specify the complete serial configuration:

```bash
./build/serial-reader /dev/ttyUSB0 \
  --baud 9600 \
  --data 7 \
  --parity even \
  --stop 1
```

To stop reading, press `Ctrl+C`.

### Supported serial parameters

| Option | Description | Default |
|---|---|---|
| `--baud RATE` | Baud rate | `9600` |
| `--data BITS` | Data bits: `5`, `6`, `7`, or `8` | `8` |
| `--parity TYPE` | Parity: `none`, `even`, or `odd` | `none` |
| `--stop BITS` | Stop bits: `1` or `2` | `1` |

The currently supported baud rates are `9600`, `19200`, `38400`, `57600`, and `115200`.

The default configuration is **9600 8N1**. The selected parameters must match the target device's configuration.

## Permissions

Linux controls access to serial devices through file permissions and system configuration.

Inspect the permissions of a device:

```bash
ls -l /dev/ttyUSB0
```

On Debian-based systems, serial devices are commonly assigned to the `dialout` group. If that is the group assigned to your device, add your user to it:

```bash
sudo usermod -aG dialout "$USER"
```

Log out and log back in for the group membership change to take effect. If your device uses a different group, use the group shown by the system.

Running the application with elevated privileges may help diagnose a permissions problem, but granting the appropriate device permissions is preferable for regular use.

## Current implementation and limitations

The current MVP is focused on serial device discovery and raw serial data reading.

Current limitations include:

- Device discovery is limited to `/dev/ttyACM*` and `/dev/ttyUSB*`.
- Only the baud rates listed above are supported.
- Received data is written as raw bytes to standard output.
- There is no built-in protocol decoding or message framing.
- There is no persistent configuration file or profile management.
- There is no unified session manager or cross-interface event timeline.
- JTAG/SWD, Modbus, and external-tool orchestration are not yet implemented.
- There is no graphical user interface.

These limitations describe the current implementation, not the full intended scope of IO Master.

## Roadmap / To-do

The roadmap will evolve as the project develops and receives feedback. The items below describe planned work; they should not be interpreted as existing features.

### Configuration and usability

- [ ] Design a persistent configuration format.
- [ ] Add configuration file loading and validation.
- [ ] Support named profiles for different devices and research projects.
- [ ] Allow common settings to be configured once and reused across invocations.
- [ ] Define predictable precedence between defaults, configuration files, profiles, and command-line overrides.
- [ ] Provide commands for inspecting and validating the active configuration.
- [ ] Improve error messages and usage documentation.

### Serial communication

- [ ] Review serial device discovery and broaden device support where appropriate.
- [ ] Improve capture options, including saving output to files.
- [ ] Add useful capture metadata without changing the raw data unexpectedly.
- [ ] Evaluate support for additional serial configurations and device-specific requirements.

### Tool integration

- [ ] Design a modular interface for integrating external tools.
- [ ] Define how IO Master launches, configures, monitors, and stops external processes.
- [ ] Investigate integration with OpenOCD for JTAG/SWD workflows.
- [ ] Investigate integration with Modbus tooling.
- [ ] Support additional tools through modular adapters where practical.
- [ ] Keep tool-specific functionality delegated to the tools that already implement it.

### Sessions and result management

- [ ] Introduce investigation sessions.
- [ ] Associate configuration profiles, devices, and tool invocations with a session.
- [ ] Record operation status, timestamps, errors, and output locations.
- [ ] Provide a consistent way to save and revisit investigation results.
- [ ] Support multiple concurrent observation tasks where appropriate.

### Event correlation

- [ ] Define a common event and result metadata model.
- [ ] Collect events from integrated tools where their interfaces permit it.
- [ ] Provide a way to compare UART, protocol, and debugging observations.
- [ ] Explore a shared timeline for events from multiple interfaces.
- [ ] Document timing limitations and distinguish observed relationships from confirmed causation.

### Quality and project maturity

- [ ] Add automated tests for configuration parsing and core functionality.
- [ ] Add tests for serial configuration validation and error handling.
- [ ] Improve build and CI checks.
- [ ] Document the architecture and integration interfaces.
- [ ] Gather feedback from IoT/OT researchers and Linux security tool maintainers.
- [ ] Evaluate packaging and distribution options, including suitability for security-focused Linux distributions.

## Contributing and feedback

IO Master is an evolving project, and feedback is welcome.

Suggestions, bug reports, feature requests, and contributions can help determine which integrations and workflow improvements are most valuable to researchers.

Please use the GitHub repository to report issues or propose improvements:

https://github.com/thekamik/io-master

In particular, feedback on configuration design, external-tool integration, session management, and cross-interface result correlation would be valuable.

## License

IO Master is free and open-source software distributed under the GNU General Public License v2.0 (GPL-2.0).

You are free to use, study, modify, and redistribute the software under the terms and conditions of the license.

Redistributed versions and derivative works must comply with the applicable GPL-2.0 requirements.

See the [`LICENSE`](LICENSE) file for the full license text.

For more information, visit the [GNU General Public License v2.0](https://www.gnu.org/licenses/old-licenses/gpl-2.0.html).
