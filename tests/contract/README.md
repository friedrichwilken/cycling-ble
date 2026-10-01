# Contract tests

These integration tests pin down the behaviour of `cycling-ble`'s public
API: golden byte vectors with their complete parsed values, the error
contract for truncated payloads, and the never-panic guarantee. They only
use the public API. See [`STABILITY.md`](../../STABILITY.md) for what the
contract covers.

## Append-only rule

Within one breaking version (a `0.x` minor series before 1.0, a major
series from 1.0 on), existing contract tests are **never edited or
deleted**. New tests may be added.

If a change to the crate makes an existing contract test fail, the change
is breaking: either rework the change, or ship it in the next breaking
version (and only then update the test).

**The one exception:** an expectation that contradicts the Bluetooth
specification or real hardware is a bug. It may be corrected in a patch
release, with an entry under "Fixed" in `CHANGELOG.md` that names the test
and the corrected value.

## Layout

| File | Covers |
|---|---|
| `main.rs` | Test entry point and the shared truncation (error-contract) helper |
| `power.rs` | Cycling Power Measurement (0x2A63) |
| `heart_rate.rs` | Heart Rate Measurement (0x2A37) |
| `csc.rs` | CSC Measurement (0x2A5B) |
| `ftms.rs` | FTMS Indoor Bike Data (0x2AD2) |
| `helpers.rs` | `revolutions_per_minute` and the public constants |
| `no_panic.rs` | Exhaustive short payloads and fixed-seed random payloads |
| `zwift_click.rs` | Zwift Click (`zwift-click` feature only; outside the contract until promoted) |

Run with `cargo test --test contract` (add `--features zwift-click` for the
Zwift Click tests).
