//! Cycling Power Measurement (0x2A63).

use crate::assert_truncations;
use cycling_ble::power::{self, CyclingPowerMeasurement, PedalPowerBalanceReference};

/// Every field of [`CyclingPowerMeasurement`], spelled out so a golden test
/// asserts the complete parsed value. Nested structs are flattened into
/// tuples:
/// - `pedal_power_balance`: `(percent, reference)`
/// - `wheel_revolutions` / `crank_revolutions`:
///   `(cumulative_revolutions, last_event_time_raw)`
/// - `extreme_force_newtons` / `extreme_torque_nm`: `(max, min)`
#[derive(Debug, Default)]
struct Expected {
    instantaneous_power_watts: i16,
    pedal_power_balance: Option<(f32, PedalPowerBalanceReference)>,
    accumulated_torque_nm: Option<f32>,
    wheel_revolutions: Option<(u32, u16)>,
    crank_revolutions: Option<(u16, u16)>,
    extreme_force_newtons: Option<(i16, i16)>,
    extreme_torque_nm: Option<(i16, i16)>,
    top_dead_spot_angle_deg: Option<u16>,
    bottom_dead_spot_angle_deg: Option<u16>,
    accumulated_energy_kj: Option<u16>,
}

fn assert_power(actual: &CyclingPowerMeasurement, e: &Expected) {
    assert_eq!(
        actual.instantaneous_power_watts, e.instantaneous_power_watts,
        "instantaneous_power_watts"
    );
    assert_eq!(
        actual.pedal_power_balance.map(|b| (b.percent, b.reference)),
        e.pedal_power_balance,
        "pedal_power_balance"
    );
    assert_eq!(
        actual.accumulated_torque_nm, e.accumulated_torque_nm,
        "accumulated_torque_nm"
    );
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
    assert_eq!(
        actual.extreme_force_newtons.map(|m| (m.max, m.min)),
        e.extreme_force_newtons,
        "extreme_force_newtons"
    );
    assert_eq!(
        actual.extreme_torque_nm.map(|m| (m.max, m.min)),
        e.extreme_torque_nm,
        "extreme_torque_nm"
    );
    assert_eq!(
        actual.top_dead_spot_angle_deg, e.top_dead_spot_angle_deg,
        "top_dead_spot_angle_deg"
    );
    assert_eq!(
        actual.bottom_dead_spot_angle_deg, e.bottom_dead_spot_angle_deg,
        "bottom_dead_spot_angle_deg"
    );
    assert_eq!(
        actual.accumulated_energy_kj, e.accumulated_energy_kj,
        "accumulated_energy_kj"
    );
}

/// Parses `data`, checks the complete value, then checks every truncation
/// errors (no shorter prefix of a Cycling Power payload with these flags is
/// itself valid).
fn golden(data: &[u8], expected: Expected) {
    let m = power::parse(data).expect("golden payload must parse");
    assert_power(&m, &expected);
    assert_truncations(data, power::parse, &[]);
}

#[test]
fn power_only() {
    golden(
        &[0x00, 0x00, 0xFA, 0x00],
        Expected {
            instantaneous_power_watts: 250,
            ..Default::default()
        },
    );
}

#[test]
fn negative_power() {
    golden(
        &[0x00, 0x00, 0xFB, 0xFF],
        Expected {
            instantaneous_power_watts: -5,
            ..Default::default()
        },
    );
}

#[test]
fn pedal_balance_left_reference() {
    golden(
        &[0x03, 0x00, 0xC8, 0x00, 0x6E],
        Expected {
            instantaneous_power_watts: 200,
            pedal_power_balance: Some((55.0, PedalPowerBalanceReference::Left)),
            ..Default::default()
        },
    );
}

#[test]
fn pedal_balance_unknown_reference() {
    golden(
        &[0x01, 0x00, 0x96, 0x00, 0x64],
        Expected {
            instantaneous_power_watts: 150,
            pedal_power_balance: Some((50.0, PedalPowerBalanceReference::Unknown)),
            ..Default::default()
        },
    );
}

#[test]
fn crank_revolution_data() {
    golden(
        &[0x20, 0x00, 0xB4, 0x00, 0xE8, 0x03, 0x00, 0x02],
        Expected {
            instantaneous_power_watts: 180,
            crank_revolutions: Some((1000, 512)),
            ..Default::default()
        },
    );
}

#[test]
fn balance_wheel_crank_angles_energy_offsets() {
    golden(
        &[
            0x31, 0x09, // flags: balance, wheel, crank, extreme angles, energy
            0x64, 0x00, // power 100 W
            0x5A, // balance 45.0 %
            0x88, 0x13, 0x00, 0x00, // wheel revolutions 5000
            0x30, 0x75, // wheel event time 30000
            0x20, 0x03, // crank revolutions 800
            0x70, 0x17, // crank event time 6000
            0xAA, 0xBB, 0xCC, // extreme angles (skipped, not exposed)
            0xFA, 0x00, // accumulated energy 250 kJ
        ],
        Expected {
            instantaneous_power_watts: 100,
            pedal_power_balance: Some((45.0, PedalPowerBalanceReference::Unknown)),
            wheel_revolutions: Some((5000, 30000)),
            crank_revolutions: Some((800, 6000)),
            accumulated_energy_kj: Some(250),
            ..Default::default()
        },
    );
}

#[test]
fn accumulated_torque() {
    golden(
        &[0x04, 0x00, 0x96, 0x00, 0x40, 0x01],
        Expected {
            instantaneous_power_watts: 150,
            accumulated_torque_nm: Some(10.0),
            ..Default::default()
        },
    );
}

#[test]
fn extreme_force_and_torque() {
    golden(
        &[
            0xC0, 0x00, // flags: extreme force, extreme torque
            0xC8, 0x00, // power 200 W
            0x52, 0x03, // force max 850
            0x88, 0xFF, // force min -120
            0x2D, 0x00, // torque max 45
            0xF6, 0xFF, // torque min -10
        ],
        Expected {
            instantaneous_power_watts: 200,
            extreme_force_newtons: Some((850, -120)),
            extreme_torque_nm: Some((45, -10)),
            ..Default::default()
        },
    );
}

#[test]
fn top_and_bottom_dead_spot_angles() {
    golden(
        &[0x00, 0x06, 0x2C, 0x01, 0x5A, 0x00, 0x0E, 0x01],
        Expected {
            instantaneous_power_watts: 300,
            top_dead_spot_angle_deg: Some(90),
            bottom_dead_spot_angle_deg: Some(270),
            ..Default::default()
        },
    );
}
