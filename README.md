# IO Master

**An evolving workflow integration tool for IoT and OT device research.**

IO Master is an open-source project aimed at making hardware and protocol research more efficient by bringing device discovery, communication, debugging tools, and investigation results into a more coherent workflow.

The project starts with serial device discovery and UART communication on Linux. Its long-term goal is to help researchers coordinate existing tools, manage reusable configurations, and correlate observations collected through different interfaces — without reinventing specialized tools that already do their jobs well.

**Project status:** Early development / MVP.

> IO Master is a work in progress. The current implementation provides serial device discovery, project management backed by JSON files, and an interactive shell for reading from a UART port. Broader workflow integration features are planned and are not yet implemented.

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

- **Serial device discovery** — list `/dev/ttyACM*` and `/dev/ttyUSB*` devices with their vendor, model, and serial number, optionally filtered by device type.
- **Project management** — create projects, list them by status, select the project to work on, and remove projects. Removed projects are kept and marked as inactive.
- **JSON configuration** — the project list and the settings of every project are stored in JSON files that are created automatically.
- **Interactive UART shell** — running a UART project opens a shell with commands to scan for devices, select the serial port, get access to it, and read from it.
- **Interruptible reading** — `Ctrl+C` stops reading and returns to the shell prompt instead of closing the program.
- **Permission helper** — the shell can run the `sudo` command that grants access to the selected serial port.
- **Damaged project detection** — a project whose settings file is missing is reported and cannot be selected or run.
- **Protocol registry** — protocols are separate modules behind one interface. UART is implemented; Bluetooth is in development.
- **Bluetooth shell (early stage)** — running a Bluetooth project opens a shell that lists the Bluetooth cards of the computer, selects the one used by the project, scans for devices in a live table, and connects and pairs devices.

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

**Status:** In progress. Projects and their settings are stored in JSON files, and the selected project and its serial port are remembered between invocations. Shared defaults, profiles, and command-line overrides are not yet implemented.

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

**Status:** In progress. Protocols are registered through a common interface (see [Source layout](#source-layout)). Adapters for external tools are not yet implemented.

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
- The `jansson` development package.

### Install dependencies on Debian, Ubuntu, or Parrot OS

```bash
sudo apt update
sudo apt install build-essential cmake pkg-config libsystemd-dev libjansson-dev
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

The build produces the `io-master` executable in the build directory:

```bash
./build/io-master --help
```

## Current usage

IO Master is organized around **projects**. A project has a name and a protocol, and its settings are stored in a JSON file instead of being passed as command-line options.

```text
io-master --help
io-master --scan [--type TYPE]
io-master --display STATUS
io-master project add NAME --protocol PROTOCOL
io-master project select ID
io-master project remove ID
io-master run [--help]
```

### Quick start

```bash
./build/io-master project add "Arduino logger" --protocol uart
./build/io-master project select 1
./build/io-master run
```

`run` opens the UART shell. A new project has no serial port yet, so the shell first lists the available devices:

```text
No serial device selected.

Available serial devices:

PORT            PRODUCENT            MODEL                          SERIAL
--------------- -------------------- ------------------------------ --------------------
/dev/ttyACM0    Arduino              UNO_WiFi_R4_CMSIS-DAP          0123456789AB

Select a device with: select PORT

Type 'help' to list commands
uart> select /dev/ttyACM0
Selected /dev/ttyACM0
Connected to /dev/ttyACM0
uart> read
Reading from /dev/ttyACM0 (press Ctrl+C to stop)
...
^C
Reading stopped
uart> exit
Closing connection...
```

### Discover serial devices

List supported devices:

```bash
./build/io-master --scan
```

Filter for USB CDC ACM devices or USB serial devices:

```bash
./build/io-master --scan --type ttyACM
./build/io-master --scan --type ttyUSB
```

The output may include:

| Field | Description |
|---|---|
| `PORT` | Serial device path |
| `PRODUCENT` | Device manufacturer, when available |
| `MODEL` | Device model, when available |
| `SERIAL` | Device serial number, when available |

Typical device paths include `/dev/ttyACM0` and `/dev/ttyUSB0`. The actual paths and metadata depend on the connected hardware and the information exposed by Linux.

### Manage projects

Create a project. The protocol must be one of the supported protocols (`uart` or `bluetooth`); its name is not case-sensitive:

```bash
./build/io-master project add "Arduino logger" --protocol uart
```

The project receives the next free numeric id and starts as active. Its settings file is created with default values.

Show the table of projects. `STATUS` is `active`, `inactive`, or `all`. The selected project is marked with `*`:

```bash
./build/io-master --display active
./build/io-master --display inactive
```

Select the project used by `run`:

```bash
./build/io-master project select 1
```

Remove a project:

```bash
./build/io-master project remove 1
```

Projects are never deleted. Removing a project marks it as inactive; inactive projects are shown with `--display inactive` and cannot be selected. Removing the selected project clears the selection. There is no command to reactivate a project yet.

### Run a project

Start the selected project:

```bash
./build/io-master run
```

`run` fails with an error when no project is selected.

Show the help of the protocol used by the selected project:

```bash
./build/io-master run --help
```

### UART shell

For a UART project, `run` starts an interactive shell:

```text
uart> help
Commands:
  help           Show this list of commands
  scan           List available serial devices
  select PORT    Select the serial port, for example /dev/ttyACM0
  permission     Get access to the selected port (asks for sudo password)
  read           Read from the serial port until Ctrl+C is pressed
  exit           Leave the shell and close the program
```

| Command | Behavior |
|---|---|
| `scan` | Lists the available `/dev/ttyACM*` and `/dev/ttyUSB*` devices. |
| `select PORT` | Stores the port in the project settings and connects to it. The port is remembered the next time the project is run. |
| `permission` | Runs `sudo chmod a+rw` on the selected port; `sudo` asks for your password in the same terminal. See [Permissions](#permissions). |
| `read` | Prints the bytes received from the port until `Ctrl+C` is pressed. Data that arrived since the previous `read` is printed first. |
| `exit` | Closes the connection and the program. `Ctrl+D` does the same. |

`Ctrl+C` stops reading and returns to the prompt; it does not close the shell.

When the project has no serial port yet, the shell lists the available devices at startup. When a port is already stored, the shell connects to it at startup and reports a missing permission or a missing device without closing.

### Configuration files

Configuration is stored in the `im_config` directory next to the executable. The directory, the project list, and the `projects` subdirectory are created automatically when the program starts and they do not exist yet:

| File | Content |
|---|---|
| `im_config/project_config.json` | List of projects and the selected project |
| `im_config/projects/<id>.json` | Settings of one project; the content depends on its protocol |

A project that is listed in `project_config.json` but has no settings file is reported as damaged. It cannot be selected or run until the file is restored.

### UART project settings

A new UART project receives a settings file with default values:

```json
{
    "device": "",
    "baudrate": 9600,
    "data_bits": 8,
    "parity": "none",
    "stop_bits": 1
}
```

| Setting | Description | Default |
|---|---|---|
| `device` | Serial device path, set by `select PORT` in the shell | empty |
| `baudrate` | Baud rate | `9600` |
| `data_bits` | Data bits: `5`, `6`, `7`, or `8` | `8` |
| `parity` | Parity: `"none"`, `"even"`, or `"odd"` | `"none"` |
| `stop_bits` | Stop bits: `1` or `2` | `1` |

The currently supported baud rates are `9600`, `19200`, `38400`, `57600`, and `115200`.

The default configuration is **9600 8N1**. The selected parameters must match the target device's configuration.

### Bluetooth shell

Bluetooth support is being built in stages. So far it provides the shell, the handling of Bluetooth cards, the scan for devices, and connecting and pairing. For a Bluetooth project, `run` starts this shell:

```text
bluetooth> help
Commands:
  help [COMMAND]       List the commands, or describe one command
  cards                List the Bluetooth cards of this computer
  select CARD          Select the Bluetooth card, for example hci0
  status               Show the selected card and its state
  scan                 Scan for Bluetooth devices until Ctrl+C is pressed
  devices              List the known devices and their IDs
  info DEVICE          Show what is known about one device
  forget               Empty the list of known devices
  connect [DEVICE]     Connect to a device
  disconnect [DEVICE]  Close the connection to a device
  pair DEVICE          Pair with a device
  unpair DEVICE        Remove the pairing with a device
  exit                 Leave the shell and close the program
```

A computer can have more than one Bluetooth card. `select CARD` stores the chosen card in the project settings, and `help COMMAND` shows the detailed help of one command.

`scan` looks for devices with the selected card and shows them in a table that is redrawn in place:

```text
Scanning on hci0 | 3 devices | 12s | press Ctrl+C to stop

 ID  ADDRESS            TYPE     ADDR     RSSI  PAIRED  STATE      NAME
  1  AA:BB:CC:00:00:01  LE       random    -66  no      visible    -
  2  AA:BB:CC:00:00:02  LE       public    -92  no      visible    Sensor
  3  AA:BB:CC:00:00:03  classic  public    -81  no      lost       Phone
```

Every address appears once: new devices are added at the bottom and known devices are updated. A device that is no longer received stays in the table as `lost`. `Ctrl+C` stops the scan, returns to the shell, and prints the devices found by the scan as a static table. When the output is not a terminal, only that final table is printed.

Every device receives an **ID**. Once the scan ends, the ID is fixed: the device keeps it in later scans until the shell is closed. The list is not stored on disk, so every session starts empty. Commands that take a device accept the ID or the Bluetooth address, for example `info 3` or `info AA:BB:CC:00:00:03`.

`devices` lists all known devices without scanning; the ones that the last scan did not see have the state `not seen`. `forget` empties the list, after which IDs start from 1 again. The list holds up to 256 devices. Devices that use random addresses change them over time and then appear as new devices with new IDs.

`connect`, `disconnect`, `pair`, and `unpair` take a device ID or an address:

```text
bluetooth> connect 2
Connecting to 2: AA:BB:CC:00:00:02 (Sensor) (press Ctrl+C to cancel)...
Connected to 2: AA:BB:CC:00:00:02 (Sensor)
```

- The device that was connected or paired last becomes the target device of the project. It is stored in the project settings, and `connect` and `disconnect` use it when no device is named.
- The system forgets devices found by a scan after about half a minute, unless they are connected or paired. Run `scan` shortly before `connect` or `pair`.
- `Ctrl+C` cancels a running connection or pairing attempt.
- A connection made in the shell is closed when the shell exits. Devices that were already connected are left alone.
- `pair` works for devices that need no code. Devices that ask for a PIN or a passkey are not supported yet, unless the desktop environment shows its own pairing dialog.
- `unpair` makes the system forget the device completely, which also disconnects it.

Cards and devices are read from BlueZ over D-Bus, so the `bluetooth` system service has to be running.

A new Bluetooth project receives this settings file:

```json
{
    "adapter": "",
    "device": ""
}
```

| Setting | Description |
|---|---|
| `adapter` | Bluetooth card, set by `select CARD` in the shell |
| `device` | Address of the target device, set by `connect` and `pair` in the shell |

Device details, pairing with a code, manual communication, and traffic monitoring are planned; see the [roadmap](#roadmap--to-do).

## Permissions

Linux controls access to serial devices through file permissions and system configuration. When the selected port cannot be opened, the UART shell reports `Permission denied`.

### Temporary access

The `permission` command of the UART shell runs:

```bash
sudo chmod a+rw -- /dev/ttyACM0
```

This gives every user of the machine read and write access to that device. The change lasts until the device is reconnected.

### Permanent access

Inspect the permissions of a device:

```bash
ls -l /dev/ttyUSB0
```

On Debian-based systems, serial devices are commonly assigned to the `dialout` group. If that is the group assigned to your device, add your user to it:

```bash
sudo usermod -aG dialout "$USER"
```

Log out and log back in for the group membership change to take effect. If your device uses a different group, use the group shown by the system.

## Source layout

| Path | Responsibility |
|---|---|
| `src/main.c` | Dispatches the parsed command |
| `src/cli/` | Command-line parser and the implementations of the commands |
| `src/project/` | Project list, selection, and project settings paths |
| `src/storage/` | Location of `im_config` and reading and writing JSON files |
| `src/discovery/` | Serial device discovery |
| `src/protocol/` | Protocol interface and the table of supported protocols |
| `src/protocol/uart/` | UART protocol: serial port access, the shell, and the shell parser |
| `src/protocol/bluetooth/` | Bluetooth protocol: access to BlueZ, cards, device scan, the shell, and the shell parser |

The headers in `include/` follow the same layout.

A protocol is described by a `protocol_t` structure (`include/protocol/protocol.h`) with its name and three functions: `init` creates the settings file of a new project, `print_help` prints the protocol help, and `run` starts the project. A new protocol is added by implementing it in `src/protocol/<name>/` and adding its structure to the table in `src/protocol/protocol.c`.

## Current implementation and limitations

The current MVP is focused on project management and raw serial data reading.

Current limitations include:

- Linux is the only supported platform.
- UART is the only complete protocol. Bluetooth can list and select Bluetooth cards, scan for devices, and connect and pair them; it cannot exchange data with them yet.
- Device discovery is limited to `/dev/ttyACM*` and `/dev/ttyUSB*`.
- Only the baud rates listed above are supported.
- The UART shell can read from the serial port but cannot write to it.
- Received data is written as raw bytes to standard output. There is no protocol decoding, message framing, or capture to a file.
- Apart from the serial port, project settings are changed by editing the JSON files.
- Projects cannot be renamed or reactivated from the command line.
- Configuration is stored next to the executable, so it is tied to the build or installation directory.
- The UART shell has no command history or line editing.
- There is no unified session manager or cross-interface event timeline.
- JTAG/SWD, Modbus, and external-tool orchestration are not yet implemented.
- There are no automated tests yet.
- There is no graphical user interface.

These limitations describe the current implementation, not the full intended scope of IO Master.

## Roadmap / To-do

The roadmap will evolve as the project develops and receives feedback. The items below describe planned work; they should not be interpreted as existing features.

### Configuration and usability

- [x] Design a persistent configuration format.
- [x] Add configuration file loading and validation.
- [ ] Add commands for changing project settings such as the baud rate.
- [ ] Support named profiles for different devices and research projects.
- [ ] Allow common settings to be configured once and reused across invocations.
- [ ] Define predictable precedence between defaults, configuration files, profiles, and command-line overrides.
- [ ] Provide commands for inspecting and validating the active configuration.
- [ ] Improve error messages and usage documentation.

### Serial communication

- [x] Provide an interactive shell for UART projects.
- [ ] Add writing to the serial port.
- [ ] Review serial device discovery and broaden device support where appropriate.
- [ ] Improve capture options, including saving output to files.
- [ ] Add useful capture metadata without changing the raw data unexpectedly.
- [ ] Evaluate support for additional serial configurations and device-specific requirements.

### Bluetooth

- [x] Add the Bluetooth protocol skeleton, its shell, and Bluetooth card selection.
- [x] Scan for devices in range, shown in a live table, with device IDs that stay fixed for the session.
- [ ] Show the services and manufacturer data of a device.
- [x] Pair, unpair, connect, and disconnect.
- [ ] Pair with devices that require a PIN or a passkey.
- [ ] Manual communication: GATT read, write, and notifications for BLE; RFCOMM and L2CAP channels for classic Bluetooth.
- [ ] Monitor the traffic between the operating system and the Bluetooth card, with capture files for Wireshark.
- [ ] Detect diagnostic devices such as Ubertooth One and use them to observe the traffic between other devices.

### Tool integration

- [x] Introduce a common interface for protocol modules.
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
