//! CSC Measurement (0x2A5B).

use crate::assert_truncations;
use cycling_ble::csc::{self, CscMeasurement};

/// Every field of [`CscMeasurement`], spelled out so a golden test asserts
/// the complete parsed value. Nested structs are flattened into
/// `(cumulative_revolutions, last_event_time_raw)` tuples.
#[derive(Debug, Default)]
struct Expected {
    wheel_revolutions: Option<(u32, u16)>,
    crank_revolutions: Option<(u16, u16)>,
}

fn assert_csc(actual: &CscMeasurement, e: &Expected) {
    assert_eq!(
        actual
            .wheel_revolutions
            .map(|w| (w.cumulative_revolutions, w.last_event_time_raw)),
        e.wheel_revolutions,
        "wheel_revolutions"
    );
    assert_eq!(
        actual
            .crank_revolutions
            .map(|c| (c.cumulative_revolutions, c.last_event_time_raw)),
        e.crank_revolutions,
        "crank_revolutions"
    );
}

/// Parses `data`, checks the complete value, then checks every truncation
/// errors.
fn golden(data: &[u8], expected: Expected) {
    let m = csc::parse(data).expect("golden payload must parse");
    assert_csc(&m, &expected);
    assert_truncations(data, csc::parse, &[]);
}

#[test]
fn no_flags_set() {
    golden(&[0x00], Expected::default());
}

#[test]
fn wheel_then_crank_offsets() {
    golden(
        &[
            0x03, // flags: wheel, crank
            0x39, 0x30, 0x00, 0x00, // wheel revolutions 12345
            0xD0, 0x07, // wheel event time 2000
            0x58, 0x02, // crank revolutions 600
            0xA0, 0x0F, // crank event time 4000
        ],
        Expected {
            wheel_revolutions: Some((12345, 2000)),
            crank_revolutions: Some((600, 4000)),
        },
    );
}
