---
name: Bug report
about: Report incorrect parsing, a panic, or other unexpected behavior
title: ""
labels: bug
assignees: ""
---

## Device

- **Device model / manufacturer**: (e.g. Wahoo KICKR CORE, Garmin HRM-Pro,
  a specific power meter model, ...)
- **Characteristic involved**: which GATT characteristic and module — e.g.
  Cycling Power Measurement (0x2A63) / `power`, Heart Rate Measurement
  (0x2A37) / `heart_rate`, CSC Measurement (0x2A5B) / `csc`, or Indoor Bike
  Data (0x2AD2) / `ftms`.

## Raw notification bytes

If at all possible, please include the **raw bytes of the notification
payload** that triggered the issue — this is the single most useful thing
you can provide, since it can be turned directly into a regression test.

You can usually capture these with a BLE sniffer (e.g. an nRF Sniffer /
Wireshark capture) or by adding a temporary debug log of the payload in
your own application before it's passed to this crate.

```
# paste raw bytes here, e.g.: [0x10, 0x00, 0xac, 0x00, 0x02, 0x01]
```

## What happened

- **Expected behavior**: what you expected the parse result to be.
- **Actual behavior**: what actually happened — an incorrect field value,
  a panic (please include the panic message/backtrace if there is one), an
  `Err(ParseError)` you didn't expect, etc.

## Environment

- `cycling-ble` version:
- Rust version (`rustc --version`):
- OS:
