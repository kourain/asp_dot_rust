# Logging

`LOGGER` publishes log entries and formatting commands to a Tokio broadcast channel. With the console sink enabled and running, a background task formats entries and writes to stdout or stderr. Calls to `with_level`, `with_format`, `with_time_format` and `with_color_output` enqueue commands for that task.

## Core concepts

| Concept | Type | Purpose |
| ------- | ---- | ------- |
| Facade | `LOGGER` (unit struct, all `fn`s are `pub fn` on the type itself) | The API you call from anywhere in the app: `LOGGER::info(...)`, `LOGGER::with_level(...)`, etc. No instance to construct or inject. |
| Formatter/writer | `Logger` | Holds the current level, format string, timestamp format and color setting, and actually writes a `LogInfo` to stdout/stderr. One instance lives inside the background task; you normally never construct one yourself. |
| Severity | `LogLevel` | The console writer compares each entry with its configured minimum. Custom receivers can implement their own filtering. |
| Transport | `LogCommand` over a `broadcast::channel` | Receivers see log entries, formatting commands and `SetEnable` notifications. The public console-sink toggles update an atomic flag directly. |

## Quick start

```rust
use asp_dot_rust::logging::{LOGGER, LogLevel};

#[tokio::main]
async fn main() {
    LOGGER::info("Starting up");
    LOGGER::with_level(LogLevel::Warn);
    LOGGER::warn("Cache miss, falling back to DB");
}
```

The console sink is enabled by default. Make the first logger call inside a Tokio runtime. `ApplicationBuilder::new()` also logs. If the first call starts the console sink outside a runtime, initialization prints a warning and leaves the running flag set. Later calls to `enable_default_logger()` then skip startup. Custom broadcast receivers can still receive entries.

## Log levels

| Level | Ordinal | Notes |
| ----- | ------- | ----- |
| `Verbose` | 0 | Lowest severity. |
| `Trace` | 1 | |
| `Debug` | 2 | The console writer suppresses this level in builds without debug assertions. Custom receivers still receive it. |
| `Info` | 3 | **Default configured level.** |
| `Warn` | 4 | |
| `Error` | 5 | Written to stderr instead of stdout. |
| `None` | 6 | As a console threshold it filters out `Verbose` through `Error`. An explicit `LOGGER::log(LogLevel::None, ...)` still meets that threshold. |

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
- The console task processes commands in channel order. Concurrent producers may interleave entries and configuration changes. A receiver that falls behind can miss commands.

## Enabling / disabling

```rust
LOGGER::disable(); // stop sending any further log messages
LOGGER::enable();  // resume logging
```

`disable()`/`enable()` control the logger globally and process-wide (not
per-request). They are safe to call multiple times and to toggle
repeatedly; logging resumes correctly after a prior `disable()`.

## Built-in request logging

Requests that finish the middleware pipeline produce an Info access log with the remote address, method, HTTP version, response status, URI and elapsed time. A framework header-size or declared body-length rejection (431 or 413) logs a warning and returns before reaching this pipeline.

```txt
[INF] 2026-10-04 12:00:00 127.0.0.1:51514 GET HTTP/1.1 200 OK /users/42 in 1.230 ms
```

With `panic = "unwind"`, the server catches a handler or middleware panic
and logs an error with the request identifiers, source location, and backtrace.
It sends HTTP 500 if the headers are still pending or aborts the stream if
the headers were already sent.

```txt
[ERR] 2026-10-04 12:00:00 Request <connection_id>:<request_id>, Unhandled panic at <file>:<line>:<column> (thread <name>): ...
Backtrace:
...
```

The server captures the backtrace at the panic site without requiring
`RUST_BACKTRACE`. A panic outside a request goes to the previous panic hook.
The request log omits the panic hook and standard
future polling frames. It keeps source locations for the remaining frames
and cuts off the backtrace at `HandleError::run` when that frame is available.

With `panic = "abort"`, Rust runs the process's existing panic hook before
terminating the process. The server leaves that hook in place.

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
use tokio::sync::broadcast::error::RecvError;

LOGGER::disable_default_logger(); // stop the built-in console sink, keep sending log commands
let mut rx = LOGGER::spawn_log_receiver();

tokio::spawn(async move {
    loop {
        match rx.recv().await {
            Ok(LogCommand::Log(entry)) => eprintln!("{}", entry.message),
            Ok(_) => continue,
            Err(RecvError::Lagged(count)) => eprintln!("Skipped {count} log commands"),
            Err(RecvError::Closed) => break,
        }
    }
});
```

`enable_default_logger()` turns the built-in console sink back on.

## Known limitations (What not work)

- No structured logging — every entry is a single formatted `String`;
  there is no key/value or JSON output mode.
- The console level applies process-wide. A custom receiver receives entries at all levels while publishing is enabled.
- The built-in access-log line does not include `request_id`/`connection_id`
  (see above) — only the panic line does.
- `disable_default_logger()` updates the console flag immediately. The console task checks it after waking to receive a command. A write already in progress can finish.
- No file rotation or file output built in — plug in your own sink (see
  above) if you need that.

## API summary

| Method | Description |
| ------ | ----------- |
| `LOGGER::trace/debug/info/warn/error/verbose(message)` | Log a message at the given level (non-blocking). |
| `LOGGER::log(level, message)` | Same, with an explicit `LogLevel`. |
| `LOGGER::with_level(level)` | Set the minimum level for the console sink. |
| `LOGGER::with_format(format)` | Set the output format string (`{level}`, `{timestamp}`, `{message}`). |
| `LOGGER::with_time_format(format)` | Set the `chrono` strftime format used for `{timestamp}`. |
| `LOGGER::with_color_output(bool)` | Enable/disable ANSI-colored level tags. |
| `LOGGER::enable()` / `disable()` | Resume/stop sending log messages process-wide. |
| `LOGGER::enable_default_logger()` / `disable_default_logger()` | Turn the built-in console sink on/off without affecting `LOGGER::log`/`enable`/`disable`. |
| `LOGGER::spawn_log_receiver()` | Get a `LogReceiver` (`broadcast::Receiver<LogCommand>`) to build a custom sink. |
