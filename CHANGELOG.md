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
