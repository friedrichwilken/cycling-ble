//! Zwift Click controller (`zwift-click` feature). Experimental: these
//! tests guard against accidental changes but the module is outside the
//! stability contract until promoted (see `STABILITY.md`).

use crate::assert_truncations;
use cycling_ble::zwift_click::{self, ClickButtonState};

/// Every field of [`ClickButtonState`] as `(plus_pressed, minus_pressed)`,
/// or `None` for a frame that carries no button state.
fn assert_click(actual: Option<ClickButtonState>, expected: Option<(bool, bool)>) {
    assert_eq!(
        actual.map(|s| (s.plus_pressed, s.minus_pressed)),
        expected,
        "(plus_pressed, minus_pressed)"
    );
}

/// Parses `data`, checks the complete value, then checks every truncation:
/// lengths in `ok_lengths` are valid shorter payloads, all others error.
fn golden(data: &[u8], expected: Option<(bool, bool)>, ok_lengths: &[usize]) {
    let m = zwift_click::parse(data).expect("golden payload must parse");
    assert_click(m, expected);
    assert_truncations(data, zwift_click::parse, ok_lengths);
}

// Button-state frames below are real-device captures. Truncated to 4..=6
// bytes they are still complete button-state frames.

#[test]
fn neither_paddle_pressed() {
    golden(
        &[0x23, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F],
        Some((false, false)),
        &[4, 5, 6],
    );
}

#[test]
fn plus_pressed() {
    golden(
        &[0x23, 0x08, 0xFF, 0xDF, 0xFF, 0xFF, 0x0F],
        Some((true, false)),
        &[4, 5, 6],
    );
}

#[test]
fn minus_pressed() {
    golden(
        &[0x23, 0x08, 0xFF, 0xFD, 0xFF, 0xFF, 0x0F],
        Some((false, true)),
        &[4, 5, 6],
    );
}

#[test]
fn both_paddles_pressed() {
    golden(
        &[0x23, 0x08, 0xFF, 0xDD, 0xFF, 0xFF, 0x0F],
        Some((true, true)),
        &[4, 5, 6],
    );
}

#[test]
fn unmapped_bits_yield_no_press() {
    golden(
        &[0x23, 0x08, 0xFF, 0xFB, 0xFF, 0xFF, 0x0F],
        Some((false, false)),
        &[4, 5, 6],
    );
}

#[test]
fn battery_level_frame_has_no_button_state() {
    // Any frame with a non-button opcode is Ok(None) once the opcode byte
    // is present.
    golden(&[0x19, 0x10, 0x64], None, &[1, 2]);
}

#[test]
fn unrecognized_opcode_has_no_button_state() {
    golden(&[0xFF, 0x00, 0x00, 0x00, 0x00], None, &[1, 2, 3, 4]);
}

#[test]
fn handshake() {
    assert_eq!(zwift_click::HANDSHAKE_REQUEST, b"RideOn");
    assert!(zwift_click::is_handshake_ack(b"RideOn\x01\x01"));
    assert!(!zwift_click::is_handshake_ack(b"nope"));
    assert!(!zwift_click::is_handshake_ack(b""));
}
