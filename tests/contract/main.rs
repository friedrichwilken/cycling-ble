//! Contract tests: the behaviour of `cycling-ble`'s public API, pinned down
//! as golden vectors. These are integration tests, so they can only reach
//! the crate through its public API.
//!
//! Append-only within one breaking version — see `README.md` in this
//! directory before editing or deleting anything here.

mod csc;
mod ftms;
mod heart_rate;
mod helpers;
mod no_panic;
mod power;
#[cfg(feature = "zwift-click")]
mod zwift_click;

use cycling_ble::ParseError;
use std::fmt::Debug;

/// Error contract: parses every truncation of `full` (lengths `0` to
/// `full.len() - 1`). Lengths listed in `ok_lengths` are themselves valid
/// payloads and must parse `Ok`; every other length must return
/// `Err(ParseError { got, needed })` with `got` equal to the truncated
/// length and `needed` greater than it.
pub(crate) fn assert_truncations<T: Debug>(
    full: &[u8],
    parse: fn(&[u8]) -> Result<T, ParseError>,
    ok_lengths: &[usize],
) {
    for len in 0..full.len() {
        let payload = &full[..len];
        match parse(payload) {
            Ok(value) => assert!(
                ok_lengths.contains(&len),
                "truncation to {len} bytes {payload:02X?} parsed Ok({value:?}), expected Err"
            ),
            Err(err) => {
                assert!(
                    !ok_lengths.contains(&len),
                    "truncation to {len} bytes {payload:02X?} returned {err:?}, expected Ok"
                );
                assert_eq!(err.got, len, "ParseError::got for {payload:02X?}");
                assert!(
                    err.needed > len,
                    "ParseError::needed ({}) must exceed got ({len}) for {payload:02X?}",
                    err.needed
                );
            }
        }
    }
}
