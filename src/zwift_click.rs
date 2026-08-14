//! Zwift Click controller — a two-paddle gear-shifter remote, distinct from
//! the Cycling Speed/Cadence/Power/HR characteristics elsewhere in this
//! crate: it exposes Zwift's own non-standard 128-bit GATT service
//! (`00000001-19CA-4651-86E5-FA29DCDD09D1`) rather than a standard Bluetooth
//! SIG characteristic, and it's a control input (button presses) rather
//! than sensor data.
//!
//! Unlike every other module here, the Click also needs one write before it
//! sends anything: on connect it stays silent until the app writes
//! [`HANDSHAKE_REQUEST`] to the service's write characteristic
//! (`00000003-19CA-4651-86E5-FA29DCDD09D1`). After that it notifies on
//! `00000002-19CA-4651-86E5-FA29DCDD09D1` (and/or indicates on
//! `00000004-19CA-4651-86E5-FA29DCDD09D1`) with the frames [`parse`]
//! decodes.
//!
//! Reverse-engineered by the community — there's no official spec. Field
//! layout cross-checked against two independent write-ups (the
//! `ajchellew/zwiftplay` GitHub project's protocol notes, and
//! `cagnulein/qdomyos-zwift`'s button-frame handling, GPL-3.0 — read for
//! the facts, reimplemented independently here rather than ported, per the
//! same policy this crate already applies to `pycycling`). The Click is
//! the simple, unencrypted member of the Zwift accessory family: unlike the
//! Play controllers, it never negotiates a session key.

use crate::reader::Reader;
use crate::ParseError;

/// Bytes to write to the Click's write characteristic
/// (`00000003-19CA-4651-86E5-FA29DCDD09D1`) once connected, to start it
/// notifying button state. Just the "RideOn" magic — no version/header
/// bytes, unlike the encrypted handshake the Play controllers use.
pub const HANDSHAKE_REQUEST: &[u8] = b"RideOn";

/// Button-state notification frame opcode.
const OPCODE_BUTTON_STATE: u8 = 0x37;
/// Idle/keepalive frame opcode, sent periodically with no button data —
/// shared across the whole Zwift accessory family, not Click-specific.
const OPCODE_IDLE: u8 = 0x15;

/// State of the Click's two paddles, decoded from a button-state
/// notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClickButtonState {
    /// The "+" paddle (gear up) is currently held down.
    pub plus_pressed: bool,
    /// The "-" paddle (gear down) is currently held down.
    pub minus_pressed: bool,
}

/// True if `data` starts with the handshake's "RideOn" magic bytes — the
/// controller echoes them back on the first notification after a
/// successful handshake write.
pub fn is_handshake_ack(data: &[u8]) -> bool {
    data.starts_with(HANDSHAKE_REQUEST)
}

/// Parses a raw notification payload from the Click's Async or SyncTx
/// characteristic.
///
/// Returns `Ok(None)` for frames that are recognized but carry no button
/// state (the idle/keepalive opcode, or any opcode this module doesn't
/// decode) rather than an error — the notify characteristic is shared
/// Zwift-accessory-family plumbing, so seeing an opcode this module
/// doesn't know about is expected, not malformed input. Returns
/// `Err(ParseError)` only when the opcode *is* [`OPCODE_BUTTON_STATE`] but
/// the payload is truncated.
pub fn parse(data: &[u8]) -> Result<Option<ClickButtonState>, ParseError> {
    let mut r = Reader::new(data);
    let opcode = r.u8()?;

    match opcode {
        OPCODE_IDLE => Ok(None),
        OPCODE_BUTTON_STATE => {
            // Byte 1: unknown/unverified meaning, skipped rather than
            // guessed at (same policy as Cycling Power's Extreme Angles
            // field).
            r.skip(1)?;
            let plus_byte = r.u8()?;
            // Byte 3: same as byte 1, unknown/unverified.
            r.skip(1)?;
            let minus_byte = r.u8()?;

            Ok(Some(ClickButtonState {
                plus_pressed: plus_byte == 0x00,
                minus_pressed: minus_byte == 0x00,
            }))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_ack_recognized() {
        assert!(is_handshake_ack(b"RideOn\x01\x01"));
        assert!(!is_handshake_ack(b"nope"));
        assert!(!is_handshake_ack(b""));
    }

    #[test]
    fn idle_frame_yields_no_button_state() {
        assert_eq!(parse(&[OPCODE_IDLE]).unwrap(), None);
    }

    #[test]
    fn unrecognized_opcode_yields_no_button_state() {
        assert_eq!(parse(&[0xFF, 0x00, 0x00, 0x00, 0x00]).unwrap(), None);
    }

    #[test]
    fn neither_paddle_pressed() {
        let data = [OPCODE_BUTTON_STATE, 0x00, 0x01, 0x00, 0x01];
        let state = parse(&data).unwrap().unwrap();
        assert_eq!(
            state,
            ClickButtonState {
                plus_pressed: false,
                minus_pressed: false,
            }
        );
    }

    #[test]
    fn plus_pressed() {
        let data = [OPCODE_BUTTON_STATE, 0x00, 0x00, 0x00, 0x01];
        let state = parse(&data).unwrap().unwrap();
        assert_eq!(
            state,
            ClickButtonState {
                plus_pressed: true,
                minus_pressed: false,
            }
        );
    }

    #[test]
    fn minus_pressed() {
        let data = [OPCODE_BUTTON_STATE, 0x00, 0x01, 0x00, 0x00];
        let state = parse(&data).unwrap().unwrap();
        assert_eq!(
            state,
            ClickButtonState {
                plus_pressed: false,
                minus_pressed: true,
            }
        );
    }

    #[test]
    fn both_paddles_pressed_at_once() {
        // Not observed in the reference implementations (their button
        // logic only ever checks one condition at a time), but nothing
        // in the frame layout rules it out, so this module decodes the
        // two paddles independently rather than assuming mutual
        // exclusion — see the module doc.
        let data = [OPCODE_BUTTON_STATE, 0x00, 0x00, 0x00, 0x00];
        let state = parse(&data).unwrap().unwrap();
        assert_eq!(
            state,
            ClickButtonState {
                plus_pressed: true,
                minus_pressed: true,
            }
        );
    }

    #[test]
    fn empty_payload_errors() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn truncated_button_state_frame_errors() {
        assert!(parse(&[OPCODE_BUTTON_STATE, 0x00, 0x00]).is_err());
    }
}
