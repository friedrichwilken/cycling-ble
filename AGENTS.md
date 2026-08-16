# AGENTS.md

Instructions for AI coding agents (and a quick-start for human contributors)
working in this repository.

## What this crate does

`cycling-ble` parses Bluetooth LE cycling GATT characteristics — Cycling
Power Measurement (0x2A63, including pedal power balance), Heart Rate
Measurement (0x2A37), CSC Measurement (0x2A5B), and FTMS Indoor Bike Data
(0x2AD2) — into typed Rust structs. It is pure byte-parsing with no BLE
transport dependency: feed it the raw notification payload from any
central-role BLE library (`btleplug`, `bluest`, ...) and get back a typed
reading. See `README.md` for the full characteristic table and an example.

## Build, test, and lint

Use the `justfile` targets rather than raw `cargo` invocations — they wrap
the underlying `cargo` commands with output already filtered down to
signal (only errors/warnings surface; a clean run prints `ok`), which
matters for keeping agent output short and worth reading.

```bash
just check   # cargo check — fast type-check, errors only
just build   # cargo build — errors only
just test    # cargo test — runs the full suite, filtered to pass/fail
             #   summary + failures (build-progress noise stripped)
just clippy  # cargo clippy --all-targets — warnings are kept; that's
             #   clippy's entire point
just fmt     # cargo fmt — applies formatting
just clean   # cargo clean — wipe build artifacts
```

Run `just check`, `just test`, and `just clippy` before considering any
change done. Run `just fmt` before committing.

## Project structure

One module per GATT characteristic under `src/`:

- `src/lib.rs` — crate root: re-exports, the shared `ParseError` type, and
  the `revolutions_per_minute` helper for turning two consecutive
  revolution-counter readings into cadence/speed.
- `src/power.rs` — Cycling Power Measurement (0x2A63).
- `src/heart_rate.rs` — Heart Rate Measurement (0x2A37).
- `src/csc.rs` — CSC Measurement (0x2A5B).
- `src/ftms.rs` — Indoor Bike Data (0x2AD2).
- `src/zwift_click.rs` — Zwift Click controller (non-standard Zwift
  accessory service, not a Bluetooth SIG characteristic). Also the one
  module with an outbound handshake constant (`HANDSHAKE_REQUEST`), since
  the device stays silent until the app writes it.
- `src/reader.rs` — internal, not part of the public API: a little-endian
  byte cursor (`Reader`) with bounds-checked reads that every parser goes
  through, since GATT payloads are sequences of optional fixed-width
  fields gated by flag bits, and an off-by-one silently misaligns every
  field after it.

Each characteristic module owns its own `parse` function, its typed
measurement struct, and its unit tests (hand-built byte sequences covering
the flag-bit combinations for that characteristic).

## Core design invariant: never panic on malformed input

This is a hard requirement, not a style preference. The bytes this crate
parses come from external BLE devices over the air — truncated
notifications, unexpected flag-bit combinations, and outright malformed
payloads are expected inputs, not edge cases to assume away. A panic here
means a crash in whatever application embedded this crate, triggered by
someone else's hardware.

Concretely:

- All multi-byte reads go through `reader::Reader`, which bounds-checks
  every read and returns `Err(ParseError)` instead of indexing/slicing
  directly into the payload.
- No `unwrap()`, `expect()`, or raw slice indexing (`data[i]`,
  `&data[a..b]`) on attacker-controlled input inside a parser. If you need
  a new field read, add it to `Reader` (or use an existing method) rather
  than reaching for raw indexing.
- When adding a new parser or field, add a test with a truncated/short
  payload asserting it returns `Err(ParseError)` rather than panicking, in
  addition to the happy-path test.
- `just clippy` is part of the check — don't skip it — but bounds-safety
  itself isn't something clippy fully catches, so review new indexing by
  hand.
