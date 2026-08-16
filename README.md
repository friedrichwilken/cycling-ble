# cycling-ble

[![CI](https://github.com/friedrichwilken/cycling-ble/actions/workflows/ci.yml/badge.svg)](https://github.com/friedrichwilken/cycling-ble/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Parsers for Bluetooth LE cycling [GATT](https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core-54/out/en/host/generic-attribute-profile--gatt-.html) (Generic Attribute Profile) characteristics — pure byte-parsing,
no BLE transport dependency. Pair it with any central-role BLE library
(e.g. [`btleplug`](https://crates.io/crates/btleplug),
[`bluest`](https://crates.io/crates/bluest)) to turn raw notification
payloads into typed readings.

Minimum supported Rust version: 1.74 (see `rust-version` in `Cargo.toml`).

## Supported characteristics

| Module | Characteristic | Notes |
|---|---|---|
| `heart_rate` | Heart Rate Measurement (0x2A37) | bpm, sensor contact, RR intervals |
| `power` | Cycling Power Measurement (0x2A63) | power, pedal power balance (L/R split), crank/wheel revolutions, torque |
| `csc` | CSC Measurement (0x2A5B) | wheel/crank revolution data |
| `ftms` | Indoor Bike Data (0x2AD2) | combined speed/cadence/power/HR, as broadcast by many smart trainers |

Also exposes `revolutions_per_minute`, a small helper for turning two
consecutive revolution-counter readings into cadence/speed, handling the
`u16` event-time counter's wraparound correctly.

## Example

```rust
use cycling_ble::power;

// `data` is the raw notification payload from characteristic 0x2A63.
let measurement = power::parse(&data)?;
println!("{} W", measurement.instantaneous_power_watts);
if let Some(balance) = measurement.pedal_power_balance {
    println!("balance: {:.1}%", balance.percent);
}
```

That's the parsing in isolation — for the full picture of where `data`
actually comes from, see
[`examples/scan_and_parse.rs`](examples/scan_and_parse.rs): a real
scan/connect/subscribe/parse flow against a nearby BLE power meter, smart
trainer, or heart rate strap, built on `btleplug`. Requires a Bluetooth
adapter and a nearby peripheral advertising the Cycling Power or Heart
Rate service. Run it with:

```sh
cargo run --example scan_and_parse
```

## Why this exists

There's no existing Rust crate that parses these cycling-specific GATT
profiles — `btleplug`/`bluest` handle BLE transport but not the bytes on
the wire for this domain. Field layouts and known real-device quirks (e.g.
the FTMS "More Data" flag's inverted presence logic on Instantaneous Speed)
were cross-checked against
[`pycycling`](https://github.com/zacharyedwardbull/pycycling) — a Python
implementation tested against real hardware — then reimplemented
independently here rather than ported, with a few deliberate correctness
fixes (e.g. Instantaneous Power, force, and torque fields are signed per
spec, not unsigned).

## Contributing

Bug reports and pull requests are welcome — see
[CONTRIBUTING.md](CONTRIBUTING.md) for how to get set up and the project's
conventions. Found a security issue? Please follow the reporting process in
[SECURITY.md](SECURITY.md) rather than opening a public issue.

## Status

Early — the public API is fully documented and covered by CI (build, test,
clippy, fmt across the MSRV), but so far exercised only against hand-built
byte sequences (see each module's tests), not yet against real hardware.
Extreme Angles (Cycling Power Measurement, a 12-bit-packed field) is parsed
far enough to skip correctly but not decoded into values — rare on consumer
power meters, low priority.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
