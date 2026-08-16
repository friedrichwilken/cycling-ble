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
//! Originally reverse-engineered from community write-ups (the
//! `ajchellew/zwiftplay` GitHub project's protocol notes, and
//! `cagnulein/qdomyos-zwift`'s button-frame handling, GPL-3.0 — read for
//! the facts, reimplemented independently here rather than ported, per the
//! same policy this crate already applies to `pycycling`), then corrected
//! against a live capture from a real Click on 2026-08-14 — the opcode and
//! frame layout below are what the hardware actually sends, not the
//! original write-ups' guesses. The Click is the simple, unencrypted
//! member of the Zwift accessory family: unlike the Play controllers, it
//! never negotiates a session key.

use crate::reader::Reader;
use crate::ParseError;

/// Bytes to write to the Click's write characteristic
/// (`00000003-19CA-4651-86E5-FA29DCDD09D1`) once connected, to start it
/// notifying button state. Just the "RideOn" magic — no version/header
/// bytes, unlike the encrypted handshake the Play controllers use.
pub const HANDSHAKE_REQUEST: &[u8] = b"RideOn";

/// Button-state notification frame opcode — verified against a live
/// capture (was incorrectly assumed to be `0x37` before the 2026-08-14
/// hardware session; real hardware never sends that value). This same
/// opcode is streamed continuously, at a high rate, whether or not a
/// paddle is held — it doubles as the family's idle/keepalive traffic
/// rather than there being a separate idle opcode, at least on the Click
/// (an `OPCODE_IDLE = 0x15` was assumed here previously by analogy with
/// other Zwift accessories' write-ups, but was never observed on real
/// Click hardware in that session).
const OPCODE_BUTTON_STATE: u8 = 0x23;

/// Bit in the button-state frame's status byte that's clear (0) while the
/// "+" paddle is held, set (1) otherwise.
const PLUS_BIT: u8 = 0b0010_0000;
/// Bit in the button-state frame's status byte that's clear (0) while the
/// "-" paddle is held, set (1) otherwise.
const MINUS_BIT: u8 = 0b0000_0010;

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
///
/// Unverified against real hardware: a 2026-08-14 live capture never
/// observed a frame matching this (the first post-handshake notification
/// was a device-info frame, opcode `0x2A`, not a bare "RideOn" echo). Kept
/// as a best-effort check — a false negative here is harmless, since
/// nothing in this module or the `zwift_click` example gates on it.
pub fn is_handshake_ack(data: &[u8]) -> bool {
    data.starts_with(HANDSHAKE_REQUEST)
}

/// Parses a raw notification payload from the Click's Async or SyncTx
/// characteristic.
///
/// Returns `Ok(None)` for frames that are recognized but carry no button
/// state (any opcode this module doesn't decode — e.g. the battery-level
/// or device-info frames also seen on this notify channel) rather than an
/// error — the notify characteristic is shared Zwift-accessory-family
/// plumbing, so seeing an opcode this module doesn't know about is
/// expected, not malformed input. Returns `Err(ParseError)` only when the
/// opcode *is* [`OPCODE_BUTTON_STATE`] but the payload is truncated.
pub fn parse(data: &[u8]) -> Result<Option<ClickButtonState>, ParseError> {
    let mut r = Reader::new(data);
    let opcode = r.u8()?;

    match opcode {
        OPCODE_BUTTON_STATE => {
            // Bytes 1-2: constant in every capture so far (0x08, 0xFF),
            // unknown/unverified meaning, skipped rather than guessed at
            // (same policy as Cycling Power's Extreme Angles field).
            r.skip(2)?;
            let status = r.u8()?;

            Ok(Some(ClickButtonState {
                plus_pressed: status & PLUS_BIT == 0,
                minus_pressed: status & MINUS_BIT == 0,
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
    fn unrecognized_opcode_yields_no_button_state() {
        assert_eq!(parse(&[0xFF, 0x00, 0x00, 0x00, 0x00]).unwrap(), None);
    }

    #[test]
    fn battery_level_frame_yields_no_button_state() {
        // Real capture: opcode 0x19, seen interleaved on the same notify
        // characteristic, unrelated to button state.
        assert_eq!(parse(&[0x19, 0x10, 0x64]).unwrap(), None);
    }

    // Byte sequences below are taken directly from a live capture against
    // real Click hardware on 2026-08-14, not hand-guessed — the crate's
    // original assumed frame layout (opcode 0x37, one whole byte per
    // paddle) never matched real hardware.

    #[test]
    fn neither_paddle_pressed() {
        let data = [OPCODE_BUTTON_STATE, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
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
        let data = [OPCODE_BUTTON_STATE, 0x08, 0xFF, 0xDF, 0xFF, 0xFF, 0x0F];
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
        let data = [OPCODE_BUTTON_STATE, 0x08, 0xFF, 0xFD, 0xFF, 0xFF, 0x0F];
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
        let data = [OPCODE_BUTTON_STATE, 0x08, 0xFF, 0xDD, 0xFF, 0xFF, 0x0F];
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
    fn unmapped_bits_yield_no_press() {
        // Real capture: single stray frames (0xFB, 0xEF) seen at rest,
        // clearing bits other than PLUS_BIT/MINUS_BIT — sensor noise, not
        // a real press. The bitmask model naturally ignores them.
        let data = [OPCODE_BUTTON_STATE, 0x08, 0xFF, 0xFB, 0xFF, 0xFF, 0x0F];
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
    fn empty_payload_errors() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn truncated_button_state_frame_errors() {
        assert!(parse(&[OPCODE_BUTTON_STATE, 0x08, 0xFF]).is_err());
    }
}
