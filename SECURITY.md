# Security Policy

## Reporting a Vulnerability

Please report security vulnerabilities using
[GitHub's private vulnerability reporting](https://github.com/friedrichwilken/cycling-ble/security/advisories/new)
(Security tab → Report a vulnerability) rather than opening a public issue.
This lets the report and any fix be discussed privately before disclosure.

If private reporting isn't available for some reason, open a regular issue
that avoids specifics (e.g. "possible parsing panic, details sent
separately") and note that you have a security-relevant finding to
discuss.

## What's in scope

`cycling-ble` parses Bluetooth LE GATT payloads (Cycling Power, Heart
Rate, CSC, and FTMS Indoor Bike Data) that originate from external,
untrusted BLE devices — a payload could come from a misbehaving,
non-conformant, or actively hostile peripheral. The crate's core
security-relevant property is that this input can never crash the
consuming application: every parser is built on bounds-checked reads
(see `src/reader.rs`), and malformed or truncated input returns a clean
`Err(ParseError)` rather than panicking (e.g. via `unwrap()`, `expect()`,
or out-of-bounds slice indexing).

If you find an input that causes a panic, an out-of-bounds read, an
integer overflow in debug builds, or otherwise violates that guarantee,
that's a security bug worth reporting — please include the raw bytes of
the payload and which characteristic/parser it was passed to.

The optional `zwift-click` feature (off by default, experimental) follows
the same bounds-checked-parsing discipline and is covered by the same
guarantee, but hasn't had the same documentation/stability pass yet.

## What's out of scope

This crate does no I/O and has no BLE transport code — it only parses
byte slices handed to it by the caller. Vulnerabilities in the BLE stack,
transport library (e.g. `btleplug`, `bluest`), or operating system
Bluetooth implementation are out of scope here; please report those
upstream.

## Future work

A `cargo-fuzz` target that continuously exercises the parsers with
arbitrary byte input would be a good addition to catch panics
automatically, but doesn't exist yet — contributions welcome.
