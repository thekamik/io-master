# IO Master

**An evolving tool for investigating IoT and OT devices, built towards AI-assisted Common Criteria evaluation.**

IO Master is an open-source project that brings device communication, AI assistance, and the record of the work into one workflow. It has two aims:

- **Device investigation.** Discover a device, talk to it over its interfaces (UART and Bluetooth today), and keep what was observed.
- **Common Criteria evaluation support.** Help an evaluator work through the work units of the Common Evaluation Methodology: find the relevant passages of the norm and of the product documentation, draft tests, guide their execution on the device, and write the record. The evaluator decides and executes; the tool prepares and documents.

The second aim is the direction of the project and is **planned, not yet implemented**. What exists today is its foundation: the device shells, the AI integration, and records kept as readable files. The design is described in [docs/architecture.md](docs/architecture.md).

**Project status:** Early development / MVP.

> IO Master is a work in progress. The current implementation provides project management, a UART shell, a Bluetooth shell, and an AI integration with saved chats. Common Criteria evaluation support and broader workflow integration are planned and are not yet implemented.

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

### Common Criteria evaluation

Evaluating a product against Common Criteria (ISO/IEC 15408) follows the Common Evaluation Methodology (ISO/IEC 18045), work unit by work unit. For each work unit the evaluator reads the methodology, follows its references into the criteria, studies the Security Target and the documentation of the product, and then either reaches a conclusion from the documents or designs and performs a test on the device. Everything has to be documented.

Much of that work is finding, reading, and recording, and software development is accelerating faster than evaluations can follow by hand. IO Master aims to automate the preparation and the record while leaving every decision with the evaluator:

- **The evaluator decides.** The tool never issues a verdict. It shows the passages it used and an estimate with its reasons.
- **The evaluator executes.** The tool proposes each command of a test; it never sends one to a device on its own. Acting on a device carries responsibility that cannot be delegated to software.
- **Everything is recorded.** The passages, the proposals, the approvals, every command, and every answer of the device become part of the evaluation record.
- **The norm is data.** Common Criteria and the methodology are loaded from replaceable index files, so a new version of the norm does not require a new version of the tool.

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

- **AI integration** — `io-master ai-config` sets up an AI provider (Anthropic Claude or Google Gemini), and a project of the `ai` protocol provides chats with the configured model. Chats are saved as readable files that can serve as records of the work. It is the first step of the planned AI-assisted analysis of protocol data. The API key is kept in the keyring of the system and is sent only over HTTPS with TLS 1.3.

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

### 7. AI-assisted Common Criteria evaluation

IO Master is intended to support the evaluator through two procedures, chosen per work unit:

- **Conceptual work units** are assessed from documents. The tool looks the work unit up in the methodology, follows its references into the criteria, lets an AI model read the Security Target, and returns the passages that matter together with an estimated probability of PASS and FAIL. The verdict stays with the evaluator.
- **Functional work units** lead to a test. The tool additionally drafts a test, as a machine-readable plan and as text for the evaluator. After the evaluator approves it, the tool guides the execution one command at a time, each executed by the evaluator, and writes a test report in which every action and every answer of the device is documented.

The architecture, the data formats, and the safeguards are described in [docs/architecture.md](docs/architecture.md).

**Status:** Planned. The device shells, the AI integration, and the saved chats that exist today are its foundation.

## Requirements

The current implementation requires:

- Linux with serial device support.
- A C compiler with C17 support, such as GCC or Clang.
- CMake 3.10 or newer.
- `pkg-config`.
- The `libsystemd` development package.
- The `jansson` development package.
- The `libcurl` development package (HTTPS requests to AI services).
- The `libsecret` development package (API keys in the system keyring).

### Install dependencies on Debian, Ubuntu, or Parrot OS

```bash
sudo apt update
sudo apt install build-essential cmake pkg-config libsystemd-dev libjansson-dev \
    libcurl4-openssl-dev libsecret-1-dev
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
io-master ai-config
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

Create a project. The protocol must be one of the supported protocols (`uart`, `bluetooth`, or `ai`); its name is not case-sensitive:

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
| `im_config/ai/ai_config.json` | AI configuration of the program, written by `io-master ai-config`; it contains no API key |
| `im_config/ai/chats/<project id>/<chat id>.json` | Saved chats of an `ai` project, one file per chat, as plain text |

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
- `pair` pairs a device that needs no code right away. When the device requires a PIN or a passkey, the shell asks for it; when both sides show a number, the shell asks to confirm that they match.
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

Device details, manual communication, and traffic monitoring are planned; see the [roadmap](#roadmap--to-do).

### AI integration

The AI integration has two parts:

- **`io-master ai-config`** (short form `io-master aic`) asks a few questions and sets up the integration: the provider, the model, and the API key. The configuration belongs to the whole program, not to a project.
- **The `ai` protocol** provides chats with the configured model. The chats of a project are saved, one file per chat, and can be listed, continued, and deleted. It is a working tool for trying out the integration that the device protocols are meant to use later.

#### Configuring the AI

```bash
./build/io-master aic
```

The command asks four questions. Every question shows its default in square brackets, and pressing Enter accepts it.

```text
AI configuration

Answer four questions to set up the AI integration.
Press Enter to accept the answer shown in [brackets].
Press Ctrl+C to cancel.

Step 1 of 4: choose the AI provider

  1) claude   Claude models by Anthropic
  2) gemini   Gemini models by Google

Provider [1]: 1

Step 2 of 4: choose the model of claude

  1) claude-opus-5-5          most capable
  2) claude-sonnet-5-5        balanced speed and capability
  3) claude-haiku-5-5         fastest and cheapest

Type a number, or the name of another model of claude.
Model [claude-opus-5-5]: 3

Step 3 of 4: the API key of claude

The key is kept in the keyring of your system, not in
a file. It is not shown while you type or paste it.

API key:
The key was stored in the keyring.

Saved: provider claude, model claude-haiku-5-5.

Step 4 of 4: test the configuration

A short message can be sent to check that the provider,
the model and the key work together. It uses a handful
of tokens.

Send a test message? [Y/n]: y

Sending a test message (press Ctrl+C to cancel)...

The configuration works: claude-haiku-5-5 answered.
```

- **Provider and model** are chosen by number; a provider can also be typed by name, and any model name of the provider is accepted, also one that is not on the list.
- **The API key** is typed without being shown. When a key is already stored, pressing Enter keeps it.
- **The test** sends one short message. On success the configuration is confirmed to work. On failure the reason is explained in plain words (a rejected key, an unknown model, no connection) and the configuration stays saved. When the test is skipped, the command says that the configuration was saved but not tested.
- **Cancelling** with `Ctrl+C`, or giving no key, before the configuration is saved changes nothing.

The answers are stored in their own file, `im_config/ai/ai_config.json`, separate from the projects. The command takes nothing on the command line; in particular an API key is never accepted there.

The less common settings have no question and are changed in that file:

| Setting | Description |
|---|---|
| `max_tokens` | Upper limit of the length of one reply. Default `16000`. |
| `system` | Instructions that apply to every conversation, or `""`. |
| `refusal_fallback` | `true` lets the service answer with another model when the chosen model declines a request. Used by `anthropic`, except for the Haiku models, which have no fallback; ignored by `gemini`. |
| `base_url` | Address of the service, for example a gateway, or `""` for the address of the provider. |
| `ca_file` | Certificate authorities to trust for `base_url` (PEM file), or `""` for the certificates of the system. |
| `key_name` | Name of the keyring entry that holds the API key. Set by the command; normally the name of the provider. |

Choosing another provider resets `base_url` and `ca_file`, so the key of one provider is never sent to the address of another.

#### Providers

| Provider | Service | Default model |
|---|---|---|
| `anthropic` (shown as `claude`) | Claude models by Anthropic | `claude-opus-5-5` |
| `gemini` | Gemini models by Google | `gemini-3.8-flash` |

The Gemini API has a free tier that includes the Flash models. Google states that content sent on the free tier is used to improve its products.

#### API key

The API key is never written to a file by IO Master and is never passed on the command line.

- The key is stored in the **keyring of the system** (the Secret Service: GNOME Keyring, KWallet), which keeps it encrypted and unlocks it at login.
- The configuration file contains only the **name** of the keyring entry (`key_name`), so it holds no secret.
- Every provider has its own key. The keyring entry is named after the provider (`anthropic`, `gemini`).
- The chat reads the key from the keyring when it starts and wipes it from memory when it ends.

The keyring protects the key on disk and from other users of the computer. It does not protect it from other programs running under the same user while the keyring is unlocked.

A key can also be stored without IO Master:

```bash
secret-tool store --label="IO Master API key: anthropic" \
    application io-master key_name anthropic
```

#### Transport security

A request that carries an API key is sent only to an `https://` address, over TLS 1.3 or newer, with the server certificate verified. Redirects are not followed. A server that offers only older TLS versions, or a certificate that cannot be verified, is refused before the key is sent.

#### Chats

```bash
./build/io-master project add "Assistant" --protocol ai
./build/io-master project select 3
./build/io-master run
```

The shell of an `ai` project has two levels: a **menu** that manages the saved chats of the project, and the **chat** itself.

```text
Provider: anthropic
Model:    claude-haiku-5-5
API key:  loaded from the keyring

  ID  UPDATED (UTC)     MSGS  MODEL                 TITLE
   1  2026-10-13 14:09     4  claude-haiku-5-5      What does 0x7E mark in HDLC?

Type 'help' to list commands
ai> open 1
Chat 1: What does 0x7E mark in HDLC?
Type a message and press Enter to send it. Press Ctrl+C to leave the chat.

you> What does 0x7E mark in HDLC?

It is the flag sequence that opens and closes every frame.

you> And how is it escaped inside a frame?
Waiting for the reply (press Ctrl+C to cancel)...

By bit stuffing: after five consecutive 1 bits a 0 is inserted.

[claude-haiku-5-5, 58 tokens in, 24 tokens out]
you> ^C
Left chat 1. It is saved with 6 messages.
ai>
```

Menu commands:

| Command | Behavior |
|---|---|
| `list` | Lists the saved chats of the project with their IDs. |
| `new [TITLE]` | Starts a new chat and enters it. Without a title, the chat is named after its first message. |
| `open ID` | Shows a saved chat and continues it. |
| `delete ID` | Deletes a saved chat after a confirmation. The file is removed for good. |
| `status` | Shows the provider, the model, and whether an API key is loaded. |
| `help`, `exit` | Lists the commands; leaves the shell. |

Inside a chat every typed line is a message for the model. `Ctrl+C` has three meanings, depending on where you are:

| Where | `Ctrl+C` |
|---|---|
| In a chat, at the `you>` prompt | Leaves the chat and returns to the menu. |
| In a chat, while a reply is awaited | Cancels the request; you stay in the chat. |
| In the menu | Starts a new line; the shell stays open (leave with `exit`). |

A chat is saved after every reply, so nothing is lost when it is left or the program ends. The ID of a deleted chat is not used again.

An AI project has no settings of its own; its chats always use the AI configuration of the program. When a saved chat is opened after another provider or model was configured, it is continued from its plain text: the new model receives what was said, without the hidden parts of the old replies.

Everything typed into a chat is sent to the configured AI service. What may be sent there is the responsibility of the user.

#### Saved chats as records

Every chat is kept in its own file:

```text
im_config/ai/chats/<project id>/<chat id>.json
```

```json
{
    "id": 1,
    "title": "What does 0x7E mark in HDLC?",
    "created": "2026-10-13T14:02:11Z",
    "updated": "2026-10-13T14:09:40Z",
    "provider": "anthropic",
    "model": "claude-haiku-5-5",
    "transcript": [
        {"role": "user", "time": "2026-10-13T14:02:11Z",
         "text": "What does 0x7E mark in HDLC?"},
        {"role": "assistant", "time": "2026-10-13T14:02:11Z",
         "model": "claude-haiku-5-5",
         "input_tokens": 21, "output_tokens": 18,
         "text": "It is the flag sequence that opens and closes every frame."}
    ],
    "provider_state": []
}
```

- `transcript` is the record meant to be read: who said what and when, which model answered, and how many tokens the exchange used. Times are in UTC. It does not depend on the AI provider.
- `provider_state` is the same conversation in the form of the provider, including parts that are not shown. The provider needs it to continue the chat exactly where it stopped.

**The chat files are plain, readable text on purpose.** A chat documents work that was carried out, and such a record has to be available as evidence, for example as an attachment to the documentation required by a quality management audit. This is a requirement, not an oversight.

The consequences for whoever uses the program:

- The files and their directory are created readable only by the user who owns them (`0600`, `0700`), but they are **not encrypted**.
- A chat can contain confidential data from the device under investigation. Store, share, and archive the files like any other confidential record.
- `delete` removes a chat for good. Copy a chat file elsewhere first when it has to be kept as a record.

Encryption of the chat files at rest may be added later.

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
| `src/cli/` | Command-line parser, the implementations of the commands, and the AI configuration questions |
| `src/project/` | Project list, selection, and project settings paths |
| `src/storage/` | Location of `im_config` and reading and writing JSON files |
| `src/discovery/` | Serial device discovery |
| `src/protocol/` | Protocol interface and the table of supported protocols |
| `src/protocol/uart/` | UART protocol: serial port access, the shell, and the shell parser |
| `src/protocol/bluetooth/` | Bluetooth protocol: access to BlueZ, cards, device scan, connecting and pairing, the shell, and the shell parser |
| `src/protocol/ai/` | AI shell: the chat menu, the chat, and the menu parser |
| `src/ai/` | AI module: the AI configuration, saved chats, the provider interface, the Anthropic and Gemini providers, HTTPS transport, and keyring access |
| `tests/` | Test scripts, kept apart from the program sources |

The headers in `include/` follow the same layout. [docs/architecture.md](docs/architecture.md) describes how the parts fit together and how the planned evaluation support builds on them.

AI services are handled the same way as protocols: each one is an `ai_provider_t` structure (`include/ai/provider.h`) registered in the table in `src/ai/ai.c`. The rest of the program calls `ai_send()` from `include/ai/ai.h` and does not depend on a particular service.

A protocol is described by a `protocol_t` structure (`include/protocol/protocol.h`) with its name and three functions: `init` creates the settings file of a new project, `print_help` prints the protocol help, and `run` starts the project. A new protocol is added by implementing it in `src/protocol/<name>/` and adding its structure to the table in `src/protocol/protocol.c`.

## Tests

Tests live in the `tests` directory, separate from the program sources. They are scripts that run the built executable and check what it does.

Run all tests after building:

```bash
ctest --test-dir build --output-on-failure
```

| Test | What it covers |
|---|---|
| `tests/bluetooth/test_bluetooth_shell.py` | The Bluetooth shell end to end: scan and device IDs, pairing with and without a code, connecting, cancelling with `Ctrl+C`, and leaving the shell |
| `tests/ai/test_ai_shell.py` | The AI configuration questions and the chat end to end, with both providers: choosing provider and model, the configuration test, storing and loading API keys, a conversation, saving, continuing and deleting chats, refusals and service errors, retries, cancelling, and the transport security rules |

The Bluetooth test needs no Bluetooth card and touches no real device. BlueZ is replaced by `tests/bluetooth/fake_bluez.py`, a stand-in that runs on a private D-Bus bus started only for the test. The test also runs a copy of the executable in a temporary directory, so the `im_config` of the build is left alone.

The AI test needs no API key and no network. The AI service is replaced by `tests/ai/fake_api.py`, a local HTTPS server with a certificate created for the test, and the system keyring by `tests/ai/fake_keyring.py` on the private bus, so the real keyring of the user is never read or changed.

The tests share helpers in `tests/testlib.py`. They require `python3`, `dbus-run-session`, `openssl`, and the Python D-Bus bindings:

```bash
sudo apt install python3-dbus python3-gi dbus-daemon openssl
```

A test can also be run directly:

```bash
python3 tests/bluetooth/test_bluetooth_shell.py build/io-master
```

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
- Common Criteria evaluation support is designed (see [docs/architecture.md](docs/architecture.md)) but not implemented.
- JTAG/SWD, Modbus, and external-tool orchestration are not yet implemented.
- Automated tests cover only the Bluetooth and AI shells.
- The AI shell supports two providers (Anthropic and Gemini) and prints a reply only when it is complete; it is not yet connected to the UART and Bluetooth protocols.
- API keys require a Secret Service keyring; there is no alternative way to supply a key.
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
- [x] Pair with devices that require a PIN or a passkey.
- [ ] Manual communication: GATT read, write, and notifications for BLE; RFCOMM and L2CAP channels for classic Bluetooth.
- [ ] Monitor the traffic between the operating system and the Bluetooth card, with capture files for Wireshark.
- [ ] Detect diagnostic devices such as Ubertooth One and use them to observe the traffic between other devices.

### AI integration

- [x] Add an AI module with a provider interface, and Anthropic (Claude) as the first provider.
- [x] Keep API keys in the system keyring and send them only over HTTPS with TLS 1.3.
- [x] Add the `ai` working shell for trying out the integration.
- [x] Add Google Gemini as a second provider.
- [x] Save chats to files, with a menu to list, continue, and delete them.
- [ ] Optional encryption of saved chats at rest.
- [x] Move the AI configuration to a program-wide `ai-config` command that asks for provider, model, and key and can test them, leaving the `ai` protocol as a chat.
- [ ] Add a provider for OpenAI-compatible services and local servers.
- [ ] Show the reply while it is being written (streaming).
- [ ] Use the AI module from the UART and Bluetooth shells to analyse captured data.

### Common Criteria evaluation support

The stages and their order are explained in [docs/architecture.md](docs/architecture.md#roadmap).

Foundations in the existing code:

- [ ] UART: sending data, not only reading.
- [ ] Action layer: a closed list of device actions that can be called without an interactive shell; the shells rebuilt on top of it.
- [ ] Machine interface of io-master: actions and results as JSON.
- [ ] AI module: answers in a fixed structure.

Stage A, norm and documents:

- [ ] Index format for Common Criteria and the methodology, as replaceable JSON files with several versions side by side.
- [ ] Importer from the published documents to the index.
- [ ] Document library for the Security Target and the product documentation.

Stage B, conceptual pipeline:

- [ ] Assessment of a work unit from documents, with cited and verified passages.
- [ ] Estimated probability of PASS and FAIL, and a record of the evaluator's verdict.
- [ ] Report of a conceptual work unit.

Stage C, functional pipeline up to approval:

- [ ] Test plan as JSON and as text, validated against the action list.
- [ ] Approval and freezing of a plan by the evaluator.

Stage D, guided execution:

- [ ] Execution of an approved plan on UART, one command at a time, each executed by the evaluator.
- [ ] Test report with every action and every answer of the device.

Stage E, breadth:

- [ ] Guided execution on Bluetooth.
- [ ] Provider for local models, for documents that may not leave the machine.
- [ ] Retrieval over document sets too large for one request.

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

- [x] Add an end-to-end test of the Bluetooth shell.
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
