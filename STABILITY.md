# Stability

What `cycling-ble` promises to keep stable, and how changes to that promise
are versioned.

## What is covered

The contract is everything listed in [`public-api.txt`](public-api.txt)
that is available with **default features**. `public-api.txt` is generated
with `cargo public-api --all-features` and committed; CI fails if the
public API drifts from it without the file being updated.

Feature-gated modules are **outside** the contract until they are promoted
to default features. Today that is `zwift_click` (feature `zwift-click`):
it appears in `public-api.txt` so changes to it are visible in review, but
it may change in any release.

## Behavioural guarantees

- **Parsers never panic**, on any input. Malformed or truncated payloads
  return `Err(ParseError)`.
- **An absent field is `None`, never a zero.** Every optional field of a
  characteristic is an `Option`. The one collection field,
  `HeartRateMeasurement::rr_intervals_secs`, is an empty `Vec` when the
  payload carries no RR intervals.
- **Units and scaling** are those stated in each field's doc comment; every
  public field documents its unit (and resolution, where scaled).
- Result structs and enums are `#[non_exhaustive]`: new fields and variants
  may be added in a non-breaking release.

## Dependencies and toolchain

- No runtime dependencies with default features.
- MSRV is Rust 1.74. Raising it is an additive (minor) change, never a
  patch: it needs at least a minor version bump.

## Versioning

The crate version is the contract version:

- **Breaking** the contract needs a breaking bump: minor while `0.x`
  (e.g. 0.2 → 0.3), major from 1.0.
- **Additions** (new items, fields, variants, features) are additive
  bumps: patch while `0.x`, minor from 1.0.
- **Fixes** are patches.

The contract is pinned down mechanically by the tests in
[`tests/contract/`](tests/contract/). Within one breaking version they are
**append-only**: existing contract tests are never edited or deleted; new
ones may be added. The one exception: an expectation that contradicts the
Bluetooth specification or real hardware is a bug, and may be corrected in
a patch release with a "Fixed" entry in [`CHANGELOG.md`](CHANGELOG.md).
