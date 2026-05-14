PHOSPHOR / ph0sphor
PHOSPHOR is a retro terminal telemetry system for turning a low-power pocket computer, primarily the Sony VAIO VGN-P19VRN, into a dedicated phosphor-style status terminal for a main workstation.
The repository, CLI namespace and binary names use the stylized form `ph0sphor`.
```text
Project name:   PHOSPHOR
Repo name:      ph0sphor
CLI namespace:  ph0sphor
Main idea:      lightweight retro TUI telemetry terminal
Client target:  Sony VAIO VGN-P series
Server target:  Windows / Linux / macOS workstation
Visual style:   phosphor CRT terminal, Pip-Boy-inspired but legally distinct
Primary goals:  security, speed, low resource usage, low network traffic
```
---
1. Project Summary
PHOSPHOR is a client-server system.
The server runs on the main computer. It collects telemetry, system state, notifications, weather, mail status and other useful information. The server normalizes and filters this data before sending it to the client.
The client runs on the Sony VAIO VGN-P19VRN. It is intentionally lightweight. It does not collect heavy data, does not parse external APIs and does not perform expensive calculations. Its job is to maintain a connection, render a terminal user interface and provide a few local time-related tools.
The user experience should feel like operating a small retro-futuristic terminal device: green or amber phosphor glow, dense pseudo-graphics, keyboard-first navigation, multiple information screens and minimal distraction.
---
2. Core Design Decision
The project must follow this rule:
```text
The server is smart.
The client is thin.
The network protocol is compact.
The default security mode is read-only.
```
This rule is mandatory. Any feature that violates it must be rejected, postponed or redesigned.
---
3. Naming Rules
Use the following naming consistently:
```text
Human-readable project name:  PHOSPHOR
Repository name:              ph0sphor
CLI command prefix:           ph0sphor
Server binary:                ph0sphor-server
Client binary:                ph0sphor-client
Control CLI binary:           ph0sphorctl
Protocol crate/module:        ph0sphor-protocol
Core crate/module:            ph0sphor-core
```
Do not use names derived directly from Fallout, Vault-Tec, Pip-Boy or any other existing franchise. The project may be aesthetically inspired by retro terminals and fictional wearable computers, but it must remain legally and visually distinct.
---
4. Target Hardware
4.1 Client Hardware
Primary client device:
```text
Sony VAIO VGN-P19VRN / VAIO P series
```
The client must be designed as if the target machine is very weak by modern standards.
Assumptions:
```text
Very low CPU performance
Limited RAM
Small physical screen
High screen resolution for its size
Weak or problematic GPU acceleration
Linux terminal environment
Keyboard-first operation
No mouse dependency
```
4.2 Server Hardware
The server runs on the user's main workstation.
Supported server operating systems should eventually include:
```text
Windows
Linux
macOS
```
The server must not noticeably affect normal workstation usage.
---
5. Main Goals
PHOSPHOR must satisfy these goals:
```text
1. Turn Sony VAIO P into a dedicated terminal-style telemetry display.
2. Keep the client extremely lightweight.
3. Keep server CPU, memory, disk and network overhead low.
4. Use a compact event-driven protocol.
5. Provide multiple switchable TUI screens.
6. Support configurable color themes.
7. Provide useful real-time telemetry from the main computer.
8. Provide mail notifications and other event notifications.
9. Provide clock, date, reminders, alarms, timer and stopwatch.
10. Provide weather information.
11. Be suitable for a public GitHub repository.
12. Be understandable to humans and language models.
13. Be secure by default.
14. Avoid unnecessary dependencies and runtime bloat.
```
---
6. Non-Goals
The following are explicit non-goals for the initial project and MVP.
Do not implement these unless the design is intentionally revised later:
```text
1. No Electron application.
2. No browser-first UI.
3. No heavy web dashboard as the main interface.
4. No 30/60 FPS animations.
5. No constant full-screen redraw loop.
6. No arbitrary remote shell execution from the client.
7. No storage of mail passwords or OAuth tokens on the client.
8. No full email body transmission by default.
9. No heavy telemetry history database on the VAIO client.
10. No mandatory cloud service.
11. No requirement for Internet access for basic telemetry.
12. No mouse-dependent UI.
13. No GPU-accelerated UI requirement on the client.
14. No feature that makes the VAIO P feel slow or unstable.
```
---
7. Recommended Technology Stack
7.1 Language
Use Rust for both server and client.
Reasoning:
```text
Rust provides native binaries.
Rust has no heavy runtime.
Rust is memory-safe by design.
Rust is suitable for low-level and networked systems.
Rust can be optimized for low CPU and memory usage.
Rust allows sharing protocol and core logic between server and client.
```
7.2 Server Runtime
Use Tokio for asynchronous networking and scheduling.
Tokio must be configured conservatively.
Server runtime rules:
```text
Do not spawn unlimited tasks.
Do not poll collectors faster than configured.
Do not use busy loops.
Do not write logs excessively.
Do not perform expensive work on every frame or tick.
Use bounded channels where possible.
Use backpressure for slow clients.
```
7.3 Client UI
Use:
```text
Ratatui for terminal UI widgets/layout
crossterm for terminal backend/input
```
Client UI rules:
```text
Render only when state changes or when a local clock element requires update.
Cap rendering to 1-2 FPS by default.
Avoid expensive animations.
Avoid unnecessary allocations inside the render loop.
Support ASCII fallback.
Support compact layout for the VAIO P screen.
```
7.4 Serialization and Protocol
Use:
```text
Primary protocol:     Protobuf binary messages
Debug protocol:       JSON dump mode for development only
Initial transport:    WebSocket with binary frames
Future transport:     Raw TCP framed protocol, optional
```
The default production path should use binary messages, not JSON.
---
8. Architecture
8.1 High-Level Architecture
```text
┌─────────────────────────────────────┐
│ Main Workstation                     │
│ ph0sphor-server                      │
│                                     │
│ ┌───────────────┐                   │
│ │ Collectors    │                   │
│ │ - CPU         │                   │
│ │ - Memory      │                   │
│ │ - Disk        │                   │
│ │ - Network     │                   │
│ │ - GPU         │                   │
│ │ - Sensors     │                   │
│ │ - Mail        │                   │
│ │ - Weather     │                   │
│ └───────┬───────┘                   │
│         │                           │
│ ┌───────▼───────┐                   │
│ │ State Store   │                   │
│ └───────┬───────┘                   │
│         │                           │
│ ┌───────▼───────┐                   │
│ │ Event Bus     │                   │
│ └───────┬───────┘                   │
│         │                           │
│ ┌───────▼───────┐                   │
│ │ Protocol API  │                   │
│ └───────┬───────┘                   │
└─────────┼───────────────────────────┘
          │ compact binary stream
          ▼
┌─────────────────────────────────────┐
│ Sony VAIO P                          │
│ ph0sphor-client                      │
│                                     │
│ ┌───────────────┐                   │
│ │ Connection    │                   │
│ │ - Auth        │                   │
│ │ - Reconnect   │                   │
│ │ - Cache       │                   │
│ └───────┬───────┘                   │
│         │                           │
│ ┌───────▼───────┐                   │
│ │ Local State   │                   │
│ └───────┬───────┘                   │
│         │                           │
│ ┌───────▼───────┐                   │
│ │ TUI Renderer  │                   │
│ └───────────────┘                   │
└─────────────────────────────────────┘
```
8.2 Server Responsibilities
The server is responsible for:
```text
1. Collecting telemetry from the main workstation.
2. Collecting optional notification sources.
3. Polling weather APIs if enabled.
4. Polling mail providers if enabled.
5. Normalizing metrics into a stable internal schema.
6. Applying thresholds and alert rules.
7. Filtering data before sending it to clients.
8. Sending full snapshots, delta updates and events.
9. Managing authentication and client pairing.
10. Protecting secrets.
11. Applying privacy rules.
12. Limiting resource usage.
```
8.3 Client Responsibilities
The client is responsible for:
```text
1. Connecting to the server.
2. Authenticating with a client token.
3. Receiving snapshots, deltas and events.
4. Maintaining local display state.
5. Rendering the TUI.
6. Switching screens.
7. Rendering themes.
8. Showing local clock/date.
9. Running local timer, stopwatch and alarm features.
10. Handling reconnects.
11. Displaying offline state.
12. Using the last cached snapshot when disconnected.
```
8.4 Client Must Not Do
The client must not:
```text
1. Poll email services directly.
2. Store email credentials.
3. Poll weather APIs directly by default.
4. Collect heavy remote telemetry itself.
5. Execute arbitrary commands on the server.
6. Maintain a large history database.
7. Perform expensive aggregation.
8. Depend on GPU acceleration.
```
---
9. Data Flow
9.1 Initial Connection
```text
1. Client starts.
2. Client loads local config.
3. Client attempts to connect to configured server.
4. Client sends hello message with protocol version and client id.
5. Server validates the client token.
6. Server sends a full snapshot.
7. Client renders HOME screen.
8. Server continues sending deltas and events.
```
9.2 Reconnect Flow
```text
1. Connection drops.
2. Client shows OFFLINE state.
3. Client keeps local clock, timer and stopwatch active.
4. Client displays last cached snapshot with stale marker.
5. Client retries connection using backoff.
6. On reconnect, client requests full snapshot.
7. Server sends full snapshot.
8. Client clears stale marker and resumes normal display.
```
9.3 Pairing Flow
```text
1. User starts server with pairing enabled.
2. User starts client without a valid token.
3. Client sends pairing request.
4. Server generates a short pairing code.
5. Client displays the pairing code.
6. User confirms the code on the server side.
7. Server issues a client token.
8. Client stores the token locally.
9. Future connections use this token.
```
---
10. Protocol Design
10.1 Message Types
The protocol must support these message types:
```text
Hello
AuthRequest
AuthResponse
PairingRequest
PairingChallenge
PairingConfirm
FullSnapshot
DeltaUpdate
Event
Ping
Pong
Error
ClientCommandRequest
ClientCommandResponse
```
`ClientCommandRequest` must be disabled or heavily restricted in MVP.
10.2 Full Snapshot
A full snapshot contains the complete current state needed to render all screens.
Full snapshots are sent:
```text
1. After successful authentication.
2. After reconnect.
3. On explicit client request.
4. Periodically as a safety mechanism.
```
Default interval:
```text
60 seconds
```
10.3 Delta Update
A delta update contains only changed fields.
Delta updates are used to reduce network traffic and client processing.
Rules:
```text
1. Do not send unchanged values.
2. Do not send high-frequency noise if it does not change visible output.
3. Coalesce updates where possible.
4. Respect configured update intervals.
5. Prefer fewer meaningful updates over many tiny updates.
```
10.4 Event
Events represent discrete changes.
Examples:
```text
New mail received
Disk usage crossed threshold
CPU temperature crossed threshold
GPU temperature crossed threshold
Server collector failed
Server collector recovered
Client reconnected
Timer completed
Alarm triggered
```
10.5 Debug JSON
A JSON debug mode may exist for development.
Rules:
```text
1. JSON debug mode is not the default production protocol.
2. JSON debug mode must not expose secrets.
3. JSON debug mode should use the same logical schema as Protobuf.
4. JSON debug mode is useful for tests, demos and manual inspection.
```
---
11. Telemetry Scope
11.1 Required Metrics for MVP
MVP must include:
```text
CPU usage percentage
RAM used/free/percentage
Disk/partition used/free/percentage
Network RX/TX speed
System uptime
Server hostname
Server OS name
Server connection status
Current date/time
Event log
```
11.2 Required Metrics After MVP
Post-MVP should include:
```text
CPU temperature
GPU usage
GPU temperature
GPU VRAM usage
SSD/HDD temperature
Disk read/write speed
Top CPU processes
Top memory processes
Battery status where available
Sensor availability status
```
11.3 Metric Availability Rule
Metric availability differs by operating system and hardware.
The system must treat unavailable metrics as normal.
Use this rule:
```text
Unavailable metric is not a fatal error.
Unavailable metric must be shown as N/A or hidden depending on UI configuration.
The server should log why the metric is unavailable.
The client should not crash because a metric is missing.
```
---
12. Suggested Collector Intervals
Default collector intervals:
```text
CPU usage:             1 second
RAM usage:             1 second
Network speed:         1 second
GPU usage/temp:        2-5 seconds
Disk usage:            10-30 seconds
Disk temperature:      30-60 seconds
Top processes:         5-10 seconds
Mail polling:          60-300 seconds
Weather polling:       10-30 minutes
Clock:                 local client update
Timer/stopwatch:       local client update
```
Collectors must be configurable.
Collectors must not run faster than necessary for the visible UI.
---
13. Performance Budget
13.1 Server Budget
Target server behavior:
```text
Idle CPU usage:           approximately 0%
Normal CPU usage:         below 1-2% on a modern workstation
Memory usage target:      below 50-100 MB where practical
Network usage target:     normally below a few KB/s
Disk writes:              minimal
Default telemetry rate:   1 Hz for primary metrics
```
The server must avoid:
```text
Busy loops
Unbounded queues
Excessive logging
Excessive disk writes
High-frequency polling of slow sensors
Repeated expensive process scans
Sending unchanged full snapshots every tick
```
13.2 Client Budget
Target VAIO client behavior:
```text
Render rate:              1-2 FPS by default
Memory usage target:      below 30-50 MB where practical
CPU usage:                as low as possible
Network usage:            normally below 1-5 KB/s
Offline usability:        required
```
The client must avoid:
```text
Heavy animations
Constant redraws
Complex layout recalculation every tick
Large history storage
Blocking network calls in UI path
Parsing large JSON payloads in production mode
```
---
14. Security Model
14.1 Default Security Rule
The server must be read-only by default.
```text
The client receives information.
The client does not control the server by default.
```
14.2 Remote Command Execution
Arbitrary remote shell execution is forbidden in MVP.
Future control commands may exist only under these rules:
```text
1. Commands must be explicitly enabled.
2. Commands must be allowlisted.
3. Commands must have stable IDs.
4. Commands must not accept arbitrary shell input by default.
5. Commands must be logged.
6. Dangerous commands must require explicit configuration.
```
14.3 Authentication
Required concepts:
```text
client_id
client_token
server_id
protocol_version
pairing_code
```
The client token must be generated by the server.
The client token must not be hardcoded in source code.
14.4 Secrets
Secrets include:
```text
Client tokens
Mail passwords
OAuth tokens
API keys
TLS private keys
```
Secret handling rules:
```text
1. Never log secrets.
2. Never send mail credentials to the client.
3. Never store mail credentials on the VAIO client.
4. Redact secrets in debug output.
5. Keep secrets in server-side config or OS credential storage where possible.
```
14.5 Mail Privacy
Mail privacy modes:
```text
count_only       Show unread count only.
sender_subject   Show sender and subject.
preview          Show sender, subject and short preview.
```
Default mode should be conservative.
Recommended default:
```text
sender_subject
```
For shared or public environments:
```text
count_only
```
Full email bodies should not be transmitted by default.
---
15. Client User Interface
15.1 UI Style
The UI must be:
```text
Terminal-based
Keyboard-first
Retro-futuristic
Readable on VAIO P
Dense but not chaotic
Low animation
Themeable
Usable without mouse
```
Visual references:
```text
CRT phosphor terminals
Amber monochrome monitors
Green monochrome monitors
Industrial diagnostic panels
Retro cyberdeck interfaces
Pip-Boy-like information density without copying protected design elements
```
15.2 Screens
The client must support multiple screens.
Required screens:
```text
HOME        Main overview
SYS         CPU/RAM/GPU overview
DISK        Storage and partitions
NET         Network state and traffic
MAIL        Mail notifications
TIME        Clock, date, timer, stopwatch, alarm
WEATHER     Weather information
LOG         Event log
ABOUT       Version, server, protocol, uptime
```
15.3 HOME Screen
HOME should display:
```text
Date
Time
Connection status
Server name
CPU summary
RAM summary
GPU summary if available
Disk summary
Mail unread count
Weather summary
Latest events
Active timer/alarm state
```
Example:
```text
╔════════════════════════ PHOSPHOR :: HOME ═══════════════════════╗
║ 2026-05-14  15:42:08        LINK: ONLINE     HOST: MAIN-PC      ║
╠═══════════════════════════════════════════════════════════════════╣
║ CPU  ██████░░░░  61%   62°C     RAM  █████░░░░░  52%            ║
║ GPU  ██░░░░░░░░  18%   54°C     SSD  ███████░░░  71%            ║
╠═══════════════════════════════════════════════════════════════════╣
║ MAIL: 3 unread       WEATHER: 17°C cloudy       TIMER: --        ║
║ EVENT: Backup completed                                          ║
╚═══════════════════════════════════════════════════════════════════╝
```
15.4 SYS Screen
SYS should display:
```text
CPU usage
CPU temperature if available
RAM usage
Swap/pagefile usage if available
GPU usage if available
GPU temperature if available
VRAM usage if available
Top CPU processes if enabled
Top RAM processes if enabled
```
15.5 DISK Screen
DISK should display:
```text
Disk/partition name
Mount point or drive letter
Used space
Free space
Usage percentage
Read/write speed if available
Temperature if available
Health status if available
```
15.6 NET Screen
NET should display:
```text
Active interface
IP address
Connection status
RX speed
TX speed
Total RX/TX counters if available
Server-client latency
Reconnect count
```
15.7 MAIL Screen
MAIL should display:
```text
Unread count
Recent messages according to privacy mode
Sender if allowed
Subject if allowed
Timestamp
Provider/account label if configured
```
MAIL must not require credentials on the client.
15.8 TIME Screen
TIME should display and support:
```text
Current local time
Current date
Timer
Stopwatch
Alarm
Reminders
```
TIME functionality should work locally on the client even when the server is disconnected.
15.9 WEATHER Screen
WEATHER should display:
```text
Current temperature
Feels-like temperature if available
Weather condition
Humidity if available
Wind if available
Short forecast
Last update time
```
Weather data should be collected by the server, not by the VAIO client by default.
15.10 LOG Screen
LOG should display:
```text
Connection events
Telemetry threshold events
Mail events
Weather update events
Collector failures
Collector recoveries
Timer/alarm events
Security/authentication events where safe
```
15.11 ABOUT Screen
ABOUT should display:
```text
PHOSPHOR version
Client version
Server version
Protocol version
Server name
Client name
Connection state
Uptime
Build target
Enabled features
```
---
16. Keyboard Controls
Default keybindings:
```text
1-9          Jump to screen
Tab          Next screen
Shift+Tab    Previous screen
C            Cycle color theme
M            Mute/unmute notifications
R            Request full snapshot
H or F1       Help
Q            Quit
/            Search or filter where applicable
Esc          Close popup/help/filter
```
The UI must remain fully usable without a mouse.
---
17. Themes
Required built-in themes:
```text
phosphor-green
amber-crt
ice-terminal
mono-lcd
high-contrast
```
Theme configuration should support:
```text
Foreground color
Background color
Accent color
Warning color
Critical color
Dim color
Border style
Progress bar style
Blinking alerts on/off
ASCII fallback on/off
```
Default theme:
```text
phosphor-green
```
Recommended alternate default for readability:
```text
amber-crt
```
---
18. Configuration
18.1 Server Config Example
```toml
[server]
bind = "0.0.0.0:7077"
name = "main-pc"
protocol = "protobuf"
debug_json = false

[security]
pairing_enabled = true
require_token = true
allow_control_commands = false

[performance]
main_tick_ms = 1000
send_deltas_only = true
full_snapshot_interval_sec = 60
max_events_in_memory = 200

[collectors.cpu]
enabled = true
interval_ms = 1000

[collectors.memory]
enabled = true
interval_ms = 1000

[collectors.network]
enabled = true
interval_ms = 1000

[collectors.disk]
enabled = true
interval_sec = 15

[collectors.gpu]
enabled = true
interval_sec = 3

[collectors.mail]
enabled = false
interval_sec = 120
privacy = "sender_subject"

[collectors.weather]
enabled = false
interval_sec = 1800
```
18.2 Client Config Example
```toml
[client]
server = "ws://main-pc.local:7077"
client_name = "vaio-p"
theme = "phosphor-green"
render_fps = 1
low_power_mode = true

[ui]
default_screen = "home"
show_scanlines = false
ascii_fallback = true
compact_mode = true

[cache]
store_last_snapshot = true
max_cached_events = 100

[keys]
next_screen = "Tab"
prev_screen = "BackTab"
theme_cycle = "C"
mute = "M"
refresh = "R"
quit = "Q"
```
---
19. Repository Structure
Recommended structure:
```text
ph0sphor/
├── README.md
├── LICENSE
├── CHANGELOG.md
├── CONTRIBUTING.md
├── SECURITY.md
├── docs/
│   ├── design.md
│   ├── protocol.md
│   ├── performance-budget.md
│   ├── security-model.md
│   ├── vaio-p-client.md
│   ├── configuration.md
│   ├── roadmap.md
│   └── screenshots/
├── crates/
│   ├── ph0sphor-core/
│   ├── ph0sphor-protocol/
│   ├── ph0sphor-server/
│   ├── ph0sphor-client/
│   └── ph0sphorctl/
├── proto/
│   └── ph0sphor.proto
├── examples/
│   ├── server.toml
│   ├── client.toml
│   ├── themes/
│   └── demo-data/
├── packaging/
│   ├── linux/
│   ├── windows/
│   └── macos/
└── .github/
    ├── workflows/
    └── ISSUE_TEMPLATE/
```
---
20. Crate Responsibilities
20.1 ph0sphor-core
Contains shared domain types and logic.
Responsibilities:
```text
Metric types
Event types
Theme types
Configuration structures
Validation helpers
Shared error types
```
20.2 ph0sphor-protocol
Contains protocol schema and encode/decode logic.
Responsibilities:
```text
Protobuf definitions
Protocol versioning
Message encoding
Message decoding
Compatibility checks
Debug JSON conversion
```
20.3 ph0sphor-server
Contains server implementation.
Responsibilities:
```text
Collectors
State store
Event bus
WebSocket endpoint
Authentication
Pairing
Privacy filtering
Configuration loading
Logging
```
20.4 ph0sphor-client
Contains VAIO client implementation.
Responsibilities:
```text
Connection handling
Reconnect logic
Local cache
TUI rendering
Screen navigation
Theme rendering
Local clock/timer/alarm tools
```
20.5 ph0sphorctl
Contains administrative CLI.
Responsibilities:
```text
Pairing confirmation
Server status inspection
Config validation
Token management
Demo data generation
Protocol debug tools
```
---
21. Roadmap
Milestone 0 — Project Skeleton
Goal:
```text
Create a clean public repository foundation.
```
Tasks:
```text
1. Create repository named ph0sphor.
2. Add README.md.
3. Add LICENSE.
4. Add SECURITY.md.
5. Add Rust workspace.
6. Add crates directory.
7. Add docs directory.
8. Add example configs.
9. Add GitHub Actions placeholder.
10. Add initial issue templates.
```
Done when:
```text
The repository clearly communicates what PHOSPHOR is and how it will be built.
```
Milestone 1 — Protocol First
Goal:
```text
Define the data contract before building UI complexity.
```
Tasks:
```text
1. Define Protobuf schema.
2. Define FullSnapshot message.
3. Define DeltaUpdate message.
4. Define Event message.
5. Define Hello/Auth messages.
6. Add protocol versioning.
7. Add test fixtures.
8. Add debug JSON dump.
```
Done when:
```text
A test can encode and decode a realistic telemetry snapshot.
```
Milestone 2 — Minimal Server
Goal:
```text
Build a working server that exposes basic telemetry.
```
Tasks:
```text
1. Implement config loading.
2. Implement CPU collector.
3. Implement memory collector.
4. Implement disk collector.
5. Implement network collector.
6. Implement state store.
7. Implement WebSocket binary endpoint.
8. Implement basic token auth stub.
9. Implement demo data generator.
```
Done when:
```text
A client or debug tool can receive live CPU/RAM/DISK/NET snapshots.
```
Milestone 3 — Minimal VAIO Client
Goal:
```text
Display live telemetry on the VAIO P in terminal UI.
```
Tasks:
```text
1. Implement WebSocket client.
2. Implement auth handshake.
3. Implement reconnect logic.
4. Implement HOME screen.
5. Implement SYS screen.
6. Implement LOG screen.
7. Implement theme support.
8. Implement screen switching.
9. Implement low-power render loop.
```
Done when:
```text
The VAIO P displays live workstation telemetry in a phosphor-style TUI.
```
Milestone 4 — Performance Pass
Goal:
```text
Make the system efficient enough for continuous use.
```
Tasks:
```text
1. Render only on state changes.
2. Send deltas instead of full snapshots where possible.
3. Add configurable collector intervals.
4. Add network usage logging.
5. Add server self-monitoring.
6. Add client self-monitoring.
7. Add low-power mode.
8. Add bounded queues.
```
Done when:
```text
Normal operation uses minimal CPU, memory and network bandwidth.
```
Milestone 5 — Security Pass
Goal:
```text
Make the default system safe for LAN usage.
```
Tasks:
```text
1. Implement pairing.
2. Implement client token storage.
3. Implement token validation.
4. Add secret redaction.
5. Confirm read-only default mode.
6. Document threat model.
7. Document mail privacy model.
8. Disable remote command execution by default.
```
Done when:
```text
A new client can be paired securely and cannot execute arbitrary server commands.
```
Milestone 6 — Useful Features
Goal:
```text
Add the features that make PHOSPHOR useful as a daily terminal panel.
```
Tasks:
```text
1. Add MAIL screen.
2. Add mail unread count.
3. Add mail privacy modes.
4. Add WEATHER screen.
5. Add TIME screen.
6. Add local timer.
7. Add local stopwatch.
8. Add local alarm.
9. Add richer event log.
```
Done when:
```text
PHOSPHOR is useful even when the user is not actively debugging the workstation.
```
Milestone 7 — VAIO P Polish
Goal:
```text
Make the client feel native to the Sony VAIO P.
```
Tasks:
```text
1. Prepare VAIO P Linux setup guide.
2. Add autostart instructions.
3. Tune layout for 1600x768.
4. Add compact mode.
5. Add ASCII fallback.
6. Add terminal font recommendations.
7. Add VAIO battery status if available.
8. Add Wi-Fi/IP status.
```
Done when:
```text
The VAIO P can boot directly into PHOSPHOR and operate like a dedicated terminal appliance.
```
Milestone 8 — Packaging and Releases
Goal:
```text
Make PHOSPHOR easy to install and test.
```
Tasks:
```text
1. Add release builds.
2. Add Linux server package.
3. Add Linux i686 client build.
4. Add Windows server build.
5. Add macOS server build.
6. Add checksums.
7. Add example configs.
8. Add demo mode.
9. Add screenshots.
10. Add installation documentation.
```
Done when:
```text
A user can download a release, configure the server and run the VAIO client.
```
---
22. MVP Definition
The first MVP is complete only when all of the following are true:
```text
1. ph0sphor-server runs on the main workstation.
2. ph0sphor-client runs on Linux on the VAIO P or equivalent low-power machine.
3. Client connects to server over a persistent connection.
4. Server sends binary telemetry messages.
5. Client displays HOME screen.
6. Client displays SYS screen.
7. Client displays LOG screen.
8. Client can switch screens using keyboard.
9. Client supports at least two themes: phosphor-green and amber-crt.
10. Client handles reconnect without crashing.
11. Server sends CPU, RAM, disk and network metrics.
12. Server has configurable collector intervals.
13. Server is read-only by default.
14. Server does not allow arbitrary remote commands.
15. README documents how to run demo mode.
```
---
23. Demo Mode
Demo mode is required for public GitHub presentation.
23.1 Server Demo Mode
```bash
ph0sphor-server --demo
```
Should generate realistic fake telemetry.
23.2 Client Demo Mode
```bash
ph0sphor-client --demo
```
Should run without a real server.
Demo mode must be useful for:
```text
Screenshots
GIF recording
README previews
Testing themes
Testing layouts
Manual QA
```
---
24. Logging
Logging rules:
```text
1. Logs must be useful but not noisy.
2. Secrets must be redacted.
3. Normal telemetry updates should not spam logs.
4. Collector failures should be logged.
5. Reconnect events should be logged.
6. Authentication failures should be logged safely.
7. Debug logging must be opt-in.
```
Recommended log levels:
```text
ERROR   Fatal or feature-breaking errors
WARN    Recoverable issues and unavailable collectors
INFO    Startup, shutdown, pairing, connection lifecycle
DEBUG   Collector details and protocol debug
TRACE   Very detailed development-only output
```
---
25. Error Handling
Error handling rules:
```text
1. The client must not crash because one metric is missing.
2. The server must not crash because one collector fails.
3. Failed collectors should enter degraded state.
4. Degraded state should be visible in LOG or ABOUT.
5. Repeated failures should be rate-limited in logs.
6. Protocol version mismatch must produce a clear error.
7. Authentication failure must produce a clear but safe error.
```
---
26. Compatibility and Versioning
Versioning rules:
```text
1. Protocol messages must contain protocol_version.
2. Server and client must expose their app version.
3. Breaking protocol changes must increment protocol major version.
4. Backward-compatible additions should preserve old fields.
5. Unknown fields should be ignored where safe.
6. Compatibility errors should be shown on ABOUT or LOG screen.
```
Recommended version command:
```bash
ph0sphor-server --version
ph0sphor-client --version
ph0sphorctl --version
```
---
27. Development Rules for Language Models and Contributors
This section is intentionally explicit for use by language models and contributors.
27.1 General Rules
```text
1. Do not change the project name.
2. Do not replace the TUI client with a web UI.
3. Do not add heavy dependencies without justification.
4. Do not move heavy work to the VAIO client.
5. Do not make JSON the default production protocol.
6. Do not implement remote shell execution in MVP.
7. Do not store mail secrets on the client.
8. Do not send full email bodies by default.
9. Do not increase render FPS without a clear reason.
10. Do not poll sensors faster than configured.
```
27.2 When Adding a Feature
Before adding a feature, answer:
```text
1. Does this feature keep the client lightweight?
2. Does this feature keep the server overhead low?
3. Does this feature preserve read-only security by default?
4. Does this feature minimize network traffic?
5. Does this feature work without a mouse?
6. Does this feature degrade gracefully when data is unavailable?
7. Does this feature fit the retro terminal concept?
```
If the answer to any of these is no, redesign the feature.
27.3 Preferred Implementation Pattern
Use this pattern:
```text
Collector -> Normalized State -> Delta/Event -> Protocol -> Client State -> TUI Widget
```
Do not use this pattern:
```text
Collector -> Raw data dump -> Client parsing -> Client business logic -> UI
```
The client must not become the data-processing layer.
---
28. Future Ideas
These are optional future features, not MVP requirements:
```text
LAN auto-discovery
TLS support
Certificate pinning
Multiple clients
Multiple servers
Custom user widgets
Script-based server widgets
Docker/Podman status
Media now-playing screen
Calendar integration
Wake-on-LAN
Read-only command status panel
Allowlisted control commands
Replay mode from recorded telemetry
Historical mini graphs
Plugin system
Raw TCP transport
Local notification sound on VAIO
```
All future features must respect the core rule:
```text
The server is smart.
The client is thin.
The network protocol is compact.
The default security mode is read-only.
```
---
29. Final Product Definition
PHOSPHOR is successful when the Sony VAIO P feels like a dedicated physical instrument rather than an old laptop running an app.
The user should be able to place the VAIO P near the main workstation and use it as:
```text
A retro telemetry terminal
A mail and notification panel
A time and reminder station
A weather/status panel
A compact system monitor
A visually distinctive cyberdeck accessory
```
The project should be technically clean enough for open-source contributors and explicit enough for language models to continue development without reinterpreting the original intent.
---
30. One-Sentence Definition
```text
PHOSPHOR / ph0sphor is a lightweight, secure, low-traffic, retro terminal telemetry system that turns a Sony VAIO P into a dedicated phosphor-style status display for a main workstation.
```
