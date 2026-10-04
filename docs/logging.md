# Logging

ASP.Rust ships a non-blocking, channel-based logger. Calling `LOGGER::info(...)`
(or any other level) never blocks the caller: the message is pushed onto a
`tokio::sync::broadcast` channel and a background task formats and writes it
to stdout/stderr. The same channel also carries configuration commands
(`with_format`, `with_level`, ...), so every `LOGGER::with_*` call is
asynchronous too — it is applied by the background task, not immediately in
the calling thread.

## Core concepts

| Concept | Type | Purpose |
| ------- | ---- | ------- |
| Facade | `LOGGER` (unit struct, all `fn`s are `pub fn` on the type itself) | The API you call from anywhere in the app: `LOGGER::info(...)`, `LOGGER::with_level(...)`, etc. No instance to construct or inject. |
| Formatter/writer | `Logger` | Holds the current level, format string, timestamp format and color setting, and actually writes a `LogInfo` to stdout/stderr. One instance lives inside the background task; you normally never construct one yourself. |
| Severity | `LogLevel` | `Verbose < Trace < Debug < Info < Warn < Error < None`, in that order. A message is written only if its level is `>=` the configured level. |
| Transport | `LogCommand` over a `broadcast::channel` | Internal enum carrying both log entries (`LogCommand::Log`) and configuration changes (`SetLogFormat`, `SetLogLevel`, `SetTimeFormat`, `SetUseColorOutput`, `SetEnable`, `SetDefaultLogger`). You don't construct these directly — the `LOGGER::with_*`/`enable`/`disable` methods do. |

## Quick start

```rust
use asp_dot_rust::logging::{LOGGER, LogLevel};

LOGGER::info("Starting up".to_string());
LOGGER::warn("Cache miss, falling back to DB".to_string());
LOGGER::error(format!("Failed to connect: {}", err));

// Only show Warn and above from here on:
LOGGER::with_level(LogLevel::Warn);
```

Logging works out of the box — you do not need to call `LOGGER::enable()`
for the default console logger to print messages. `enable()`/`disable()`
(see below) are only needed if you want to explicitly pause/resume all
logging at runtime.

## Log levels

| Level | Ordinal | Notes |
| ----- | ------- | ----- |
| `Verbose` | 0 | Lowest severity. |
| `Trace` | 1 | |
| `Debug` | 2 | Always suppressed in release builds (`cfg(not(debug_assertions))`), regardless of the configured level. |
| `Info` | 3 | **Default configured level.** |
| `Warn` | 4 | |
| `Error` | 5 | Written to stderr instead of stdout. |
| `None` | 6 | Not used as a message level — pass it to `with_level(LogLevel::None)` to silence every message (nothing is ever `>= None`). |

## Configuring output

```rust
use asp_dot_rust::logging::{LOGGER, LogLevel};

LOGGER::with_level(LogLevel::Debug);
LOGGER::with_format("[{level}] {timestamp} {message}".to_string()); // current default
LOGGER::with_time_format("%Y-%m-%dT%H:%M:%S%.3f".to_string());
LOGGER::with_color_output(false); // e.g. when writing to a file/non-TTY
```

- `with_format` only recognizes three placeholders: `{level}`, `{timestamp}`,
  `{message}`. Anything else in the string (including `{requestid}` /
  `{connectionid}`, which existed in an earlier version) is treated as
  literal text and printed as-is.
- Because every `with_*` call goes through the same channel as log
  messages, a call immediately followed by a log line is not guaranteed to
  apply to that exact line under concurrent load from multiple
  tasks/threads — treat configuration as "effective shortly after the
  call", set it once near startup rather than toggling it per-request.

## Enabling / disabling

```rust
LOGGER::disable(); // stop sending any further log messages
LOGGER::enable();  // resume logging
```

`disable()`/`enable()` control the logger globally and process-wide (not
per-request). They are safe to call multiple times and to toggle
repeatedly; logging resumes correctly after a prior `disable()`.

## Built-in request logging

Every HTTP request already produces one access-log line automatically,
after the middleware pipeline finishes:

```txt
[INF] 2026-10-04 12:00:00 127.0.0.1:51514 GET HTTP/1.1 /users/42 200 OK in 1.230 ms
```

If a handler or middleware panics, the panic is caught and logged
separately as an error, including the request's identifiers, before a
500 response is returned:

```txt
[ERR] 2026-10-04 12:00:00 Request <connection_id>:<request_id>, Unhandled panic: ...
```

`HttpContext::request.request_id` (`Ulid`, one per request) and
`.connection_id` (`Ulid`, shared by every request on the same keep-alive
connection) are available if you want to log them yourself from a
middleware or handler — currently the built-in access-log line above does
**not** include them, only the panic line does.

## Writing your own log sink

To send logs somewhere other than (or in addition to) the console — a
file, a log collector, etc. — subscribe your own receiver instead of (or
alongside) the built-in console sink:

```rust
use asp_dot_rust::logging::{LOGGER, LogCommand};

LOGGER::disable_default_logger(); // stop the built-in console sink, keep sending log commands
let mut rx = LOGGER::spawn_log_receiver();

tokio::spawn(async move {
    while let Ok(LogCommand::Log(entry)) = rx.recv().await {
        // write `entry` (LogInfo { timestamp, level, message }) wherever you want
    }
});
```

`enable_default_logger()` turns the built-in console sink back on.

## Known limitations (What not work)

- No structured logging — every entry is a single formatted `String`;
  there is no key/value or JSON output mode.
- No per-module or per-target log level — `with_level` is one global
  threshold for the whole process.
- The built-in access-log line does not include `request_id`/`connection_id`
  (see above) — only the panic line does.
- `disable_default_logger()` does not take effect instantly: the
  background task only notices the change when it next receives a log
  command, so one more message may still be printed right after calling it.
- No file rotation or file output built in — plug in your own sink (see
  above) if you need that.

## API summary

| Method | Description |
| ------ | ----------- |
| `LOGGER::trace/debug/info/warn/error/verbose(message)` | Log a message at the given level (non-blocking). |
| `LOGGER::log(level, message)` | Same, with an explicit `LogLevel`. |
| `LOGGER::with_level(level)` | Set the minimum level that gets written. |
| `LOGGER::with_format(format)` | Set the output format string (`{level}`, `{timestamp}`, `{message}`). |
| `LOGGER::with_time_format(format)` | Set the `chrono` strftime format used for `{timestamp}`. |
| `LOGGER::with_color_output(bool)` | Enable/disable ANSI-colored level tags. |
| `LOGGER::enable()` / `disable()` | Resume/stop sending log messages process-wide. |
| `LOGGER::enable_default_logger()` / `disable_default_logger()` | Turn the built-in console sink on/off without affecting `LOGGER::log`/`enable`/`disable`. |
| `LOGGER::spawn_log_receiver()` | Get a `LogReceiver` (`broadcast::Receiver<LogCommand>`) to build a custom sink. |
