# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing has been published to crates.io yet — everything below is
pre-release.

### Added

- Initial `cycling-ble` crate: pure byte-parsing (no BLE transport
  dependency) for four cycling GATT characteristics — Cycling Power
  Measurement (0x2A63, including pedal power balance), Heart Rate
  Measurement (0x2A37), CSC Measurement (0x2A5B), and FTMS Indoor Bike
  Data (0x2AD2) — plus a `revolutions_per_minute` helper for turning two
  consecutive revolution-counter readings into cadence/speed.
- `justfile` with filtered `check`/`build`/`test`/`clippy`/`fmt`/`clean`
  targets.
- Zwift Click controller support (`zwift_click` module): handshake
  request, button-state parsing (`ClickButtonState`), and battery/idle
  frame handling for the non-standard Zwift accessory service.

### Fixed

- `zwift_click::parse` — the button-state opcode, byte layout, and
  press-encoding were all wrong, verified against a live capture from real
  Click hardware (2026-08-14): the opcode is `0x23` (not the previously
  assumed `0x37`), the two paddles share one status byte at offset 3 (not
  two separate whole bytes), and presses are signaled by individual bits
  going low (`0x20` for "+", `0x02` for "-") rather than a whole byte
  going to `0x00`. The crate previously never detected a real button press
  on actual hardware. Also fixed `examples/zwift_click.rs`'s device
  matching, which filtered on the wrong service UUID (the one exposed
  post-connection, not the one actually advertised) and so never found a
  real Click either.
