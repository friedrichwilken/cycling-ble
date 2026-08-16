//! Parsers for Bluetooth LE cycling GATT characteristics: Cycling Power
//! (including pedal power balance), Heart Rate, CSC, and FTMS Indoor Bike
//! Data.
//!
//! Pure byte-parsing with no BLE transport dependency — feed it the raw
//! notification payload from any central-role BLE library (`btleplug`,
//! `bluest`, ...) and get back a typed reading. Field layouts and known
//! real-device quirks were cross-checked against `pycycling`
//! (<https://github.com/zacharyedwardbull/pycycling>, MIT licensed), a
//! Python implementation tested against real hardware; this crate
//! reimplements them independently rather than porting its code.

#![deny(missing_docs)]

mod reader;

/// CSC (Cycling Speed and Cadence) Measurement (characteristic 0x2A5B).
pub mod csc;
/// FTMS Indoor Bike Data (characteristic 0x2AD2).
pub mod ftms;
/// Heart Rate Measurement (characteristic 0x2A37).
pub mod heart_rate;
/// Cycling Power Measurement (characteristic 0x2A63).
pub mod power;
/// Zwift Click controller (non-standard Zwift accessory service).
///
/// Experimental and undocumented elsewhere — opt in with the `zwift-click`
/// feature. Not enabled by default.
#[cfg(feature = "zwift-click")]
pub mod zwift_click;

pub use csc::CscMeasurement;
pub use ftms::IndoorBikeData;
pub use heart_rate::HeartRateMeasurement;
pub use power::CyclingPowerMeasurement;
#[cfg(feature = "zwift-click")]
pub use zwift_click::ClickButtonState;

/// A characteristic payload was shorter than its flags said it should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseError {
    /// Minimum payload length, in bytes, required to parse the fields
    /// indicated by the flags already read.
    pub needed: usize,
    /// Actual payload length, in bytes.
    pub got: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "payload too short: needed at least {} bytes, got {}",
            self.needed, self.got
        )
    }
}

impl std::error::Error for ParseError {}

/// Event-time resolution (Hz) for Cycling Power's crank revolution data and
/// CSC's crank revolution data — both 1/1024s.
pub const CRANK_EVENT_TIME_HZ: f32 = 1024.0;
/// Event-time resolution (Hz) for CSC's wheel revolution data — 1/1024s.
/// Distinct from Cycling Power's wheel event time; see
/// [`CP_WHEEL_EVENT_TIME_HZ`].
pub const CSC_WHEEL_EVENT_TIME_HZ: f32 = 1024.0;
/// Event-time resolution (Hz) for Cycling Power's wheel revolution data —
/// 1/2048s. Same field name as CSC's, different resolution; each GATT
/// service defines its own.
pub const CP_WHEEL_EVENT_TIME_HZ: f32 = 2048.0;

/// Wraparound modulus for 16-bit cumulative revolution counters — Cycling
/// Power's and CSC's crank revolution data. Pass to
/// [`revolutions_per_minute`]'s `revolutions_wrap_at` parameter.
pub const CRANK_REVOLUTIONS_WRAP_AT: u64 = 1 << 16;
/// Wraparound modulus for 32-bit cumulative revolution counters — Cycling
/// Power's and CSC's wheel revolution data. Pass to
/// [`revolutions_per_minute`]'s `revolutions_wrap_at` parameter.
pub const WHEEL_REVOLUTIONS_WRAP_AT: u64 = 1 << 32;

/// Computes cadence/speed in revolutions per minute from two consecutive
/// revolution-counter readings (Cycling Power's or CSC's wheel/crank
/// revolution data), handling wraparound of both the event-time counter
/// (always `u16`) and the cumulative-revolution counter (`u16` for crank
/// data, `u32` for wheel data — width given by `revolutions_wrap_at`).
///
/// Returns `None` when `curr` isn't actually newer than `prev` — a
/// duplicate or out-of-order notification, or zero elapsed revolutions/time
/// — rather than a bogus rate. Pass [`CRANK_EVENT_TIME_HZ`],
/// [`CSC_WHEEL_EVENT_TIME_HZ`], or [`CP_WHEEL_EVENT_TIME_HZ`] for
/// `event_time_resolution_hz`, and [`CRANK_REVOLUTIONS_WRAP_AT`] or
/// [`WHEEL_REVOLUTIONS_WRAP_AT`] for `revolutions_wrap_at`, depending on
/// which field you're computing.
pub fn revolutions_per_minute(
    prev_cumulative_revolutions: u32,
    prev_event_time_raw: u16,
    curr_cumulative_revolutions: u32,
    curr_event_time_raw: u16,
    revolutions_wrap_at: u64,
    event_time_resolution_hz: f32,
) -> Option<f32> {
    let prev_revs = prev_cumulative_revolutions as u64 % revolutions_wrap_at;
    let curr_revs = curr_cumulative_revolutions as u64 % revolutions_wrap_at;
    let delta_revs = if curr_revs >= prev_revs {
        curr_revs - prev_revs
    } else {
        curr_revs + revolutions_wrap_at - prev_revs
    };
    if delta_revs == 0 {
        return None;
    }
    let delta_ticks = curr_event_time_raw.wrapping_sub(prev_event_time_raw);
    if delta_ticks == 0 {
        return None;
    }
    let delta_secs = delta_ticks as f32 / event_time_resolution_hz;
    Some(delta_revs as f32 / delta_secs * 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_basic() {
        // 10 revolutions in 1024 ticks (1s at 1024Hz) = 10 rpm... wait, 10
        // revs/second = 600 rpm. Sanity-check the math directly.
        let rpm = revolutions_per_minute(
            0,
            0,
            10,
            1024,
            CRANK_REVOLUTIONS_WRAP_AT,
            CRANK_EVENT_TIME_HZ,
        )
        .unwrap();
        assert!((rpm - 600.0).abs() < 0.01);
    }

    #[test]
    fn rpm_handles_event_time_wraparound() {
        // prev near u16::MAX, curr wrapped around to a small value.
        let prev_time = u16::MAX - 100; // 65435
        let curr_time = 923u16; // 65435 + 1024, wrapped past 65536
        let rpm = revolutions_per_minute(
            0,
            prev_time,
            10,
            curr_time,
            CRANK_REVOLUTIONS_WRAP_AT,
            CRANK_EVENT_TIME_HZ,
        )
        .unwrap();
        assert!((rpm - 600.0).abs() < 0.01);
    }

    #[test]
    fn rpm_handles_crank_revolution_wraparound() {
        // 16-bit crank counter wraps from near 65536 back to a small value:
        // prev=65531, curr=5 is 10 revolutions elapsed, not a rollback.
        let prev_revs = (u16::MAX as u32) - 4;
        let curr_revs = 5u32;
        let rpm = revolutions_per_minute(
            prev_revs,
            0,
            curr_revs,
            1024,
            CRANK_REVOLUTIONS_WRAP_AT,
            CRANK_EVENT_TIME_HZ,
        )
        .unwrap();
        assert!((rpm - 600.0).abs() < 0.01);
    }

    #[test]
    fn rpm_handles_wheel_revolution_wraparound() {
        // Same wraparound scenario, but at the wheel counter's 32-bit width.
        let prev_revs = u32::MAX - 4;
        let curr_revs = 5u32;
        let rpm = revolutions_per_minute(
            prev_revs,
            0,
            curr_revs,
            2048,
            WHEEL_REVOLUTIONS_WRAP_AT,
            CP_WHEEL_EVENT_TIME_HZ,
        )
        .unwrap();
        assert!((rpm - 600.0).abs() < 0.01);
    }

    #[test]
    fn rpm_none_on_duplicate_notification() {
        assert_eq!(
            revolutions_per_minute(
                5,
                100,
                5,
                100,
                CRANK_REVOLUTIONS_WRAP_AT,
                CRANK_EVENT_TIME_HZ
            ),
            None
        );
    }

    #[test]
    fn rpm_none_on_zero_elapsed_time() {
        assert_eq!(
            revolutions_per_minute(
                5,
                100,
                6,
                100,
                CRANK_REVOLUTIONS_WRAP_AT,
                CRANK_EVENT_TIME_HZ
            ),
            None
        );
    }
}
