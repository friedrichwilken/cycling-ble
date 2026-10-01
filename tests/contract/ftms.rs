//! FTMS Indoor Bike Data (0x2AD2).

use crate::assert_truncations;
use cycling_ble::ftms::{self, IndoorBikeData};

/// Every field of [`IndoorBikeData`], spelled out so a golden test asserts
/// the complete parsed value.
#[derive(Debug, Default)]
struct Expected {
    instantaneous_speed: Option<f32>,
    average_speed: Option<f32>,
    instantaneous_cadence: Option<f32>,
    average_cadence: Option<f32>,
    total_distance: Option<u32>,
    resistance_level: Option<i16>,
    instantaneous_power: Option<i16>,
    average_power: Option<i16>,
    total_energy: Option<u16>,
    energy_per_hour: Option<u16>,
    energy_per_minute: Option<u8>,
    heart_rate_bpm: Option<u8>,
    metabolic_equivalent: Option<f32>,
    elapsed_time_secs: Option<u16>,
    remaining_time_secs: Option<u16>,
}

fn assert_indoor_bike_data(actual: &IndoorBikeData, e: &Expected) {
    assert_eq!(
        actual.instantaneous_speed, e.instantaneous_speed,
        "instantaneous_speed"
    );
    assert_eq!(actual.average_speed, e.average_speed, "average_speed");
    assert_eq!(
        actual.instantaneous_cadence, e.instantaneous_cadence,
        "instantaneous_cadence"
    );
    assert_eq!(actual.average_cadence, e.average_cadence, "average_cadence");
    assert_eq!(actual.total_distance, e.total_distance, "total_distance");
    assert_eq!(
        actual.resistance_level, e.resistance_level,
        "resistance_level"
    );
    assert_eq!(
        actual.instantaneous_power, e.instantaneous_power,
        "instantaneous_power"
    );
    assert_eq!(actual.average_power, e.average_power, "average_power");
    assert_eq!(actual.total_energy, e.total_energy, "total_energy");
    assert_eq!(actual.energy_per_hour, e.energy_per_hour, "energy_per_hour");
    assert_eq!(
        actual.energy_per_minute, e.energy_per_minute,
        "energy_per_minute"
    );
    assert_eq!(actual.heart_rate_bpm, e.heart_rate_bpm, "heart_rate_bpm");
    assert_eq!(
        actual.metabolic_equivalent, e.metabolic_equivalent,
        "metabolic_equivalent"
    );
    assert_eq!(
        actual.elapsed_time_secs, e.elapsed_time_secs,
        "elapsed_time_secs"
    );
    assert_eq!(
        actual.remaining_time_secs, e.remaining_time_secs,
        "remaining_time_secs"
    );
}

/// Parses `data`, checks the complete value, then checks every truncation
/// errors.
fn golden(data: &[u8], expected: Expected) {
    let m = ftms::parse(data).expect("golden payload must parse");
    assert_indoor_bike_data(&m, &expected);
    assert_truncations(data, ftms::parse, &[]);
}

#[test]
fn more_data_clear_means_speed_present() {
    golden(
        &[0x00, 0x00, 0xC4, 0x09],
        Expected {
            instantaneous_speed: Some(25.0),
            ..Default::default()
        },
    );
}

#[test]
fn more_data_set_means_speed_absent() {
    golden(
        &[0x41, 0x00, 0xFA, 0x00],
        Expected {
            instantaneous_power: Some(250),
            ..Default::default()
        },
    );
}

#[test]
fn cadence_half_rpm_resolution() {
    golden(
        &[0x05, 0x00, 0xAA, 0x00],
        Expected {
            instantaneous_cadence: Some(85.0),
            ..Default::default()
        },
    );
}

#[test]
fn cadence_power_hr_elapsed_offsets() {
    golden(
        &[
            0x45, 0x0A, // flags: more data, cadence, power, HR, elapsed time
            0xB4, 0x00, // cadence 90.0 rpm
            0x2C, 0x01, // power 300 W
            0x91, // HR 145 bpm
            0x10, 0x0E, // elapsed 3600 s
        ],
        Expected {
            instantaneous_cadence: Some(90.0),
            instantaneous_power: Some(300),
            heart_rate_bpm: Some(145),
            elapsed_time_secs: Some(3600),
            ..Default::default()
        },
    );
}

#[test]
fn total_distance_u24() {
    golden(
        &[0x11, 0x00, 0x40, 0x42, 0x0F],
        Expected {
            total_distance: Some(1_000_000),
            ..Default::default()
        },
    );
}

#[test]
fn average_speed_and_cadence() {
    golden(
        &[0x0B, 0x00, 0x08, 0x07, 0xA0, 0x00],
        Expected {
            average_speed: Some(18.0),
            average_cadence: Some(80.0),
            ..Default::default()
        },
    );
}

#[test]
fn resistance_level_and_average_power() {
    golden(
        &[0xA1, 0x00, 0xFD, 0xFF, 0xDC, 0x00],
        Expected {
            resistance_level: Some(-3),
            average_power: Some(220),
            ..Default::default()
        },
    );
}

#[test]
fn expended_energy_block() {
    golden(
        &[0x01, 0x01, 0xF4, 0x01, 0x58, 0x02, 0x0A],
        Expected {
            total_energy: Some(500),
            energy_per_hour: Some(600),
            energy_per_minute: Some(10),
            ..Default::default()
        },
    );
}

#[test]
fn metabolic_equivalent_and_remaining_time() {
    golden(
        &[0x01, 0x14, 0x55, 0xB0, 0x04],
        Expected {
            metabolic_equivalent: Some(8.5),
            remaining_time_secs: Some(1200),
            ..Default::default()
        },
    );
}
