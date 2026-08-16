use crate::reader::Reader;
use crate::ParseError;

/// Which leg a [`PedalPowerBalance::percent`] value refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PedalPowerBalanceReference {
    /// The device reported a balance split but didn't say which leg it's
    /// measured from.
    Unknown,
    /// The percentage is measured from the left pedal.
    Left,
}

/// Split of instantaneous power between the two pedals, if the power meter
/// supports left/right balance measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PedalPowerBalance {
    /// 0.0-100.0, meaning given by `reference`.
    pub percent: f32,
    /// Which leg `percent` is measured from.
    pub reference: PedalPowerBalanceReference,
}

/// A paired maximum/minimum value, as reported by Cycling Power's Extreme
/// Force and Extreme Torque fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinMax<T> {
    /// The largest value seen over the reporting period.
    pub max: T,
    /// The smallest value seen over the reporting period.
    pub min: T,
}

/// Cumulative wheel revolution count and the timestamp of the last one, as
/// reported by Cycling Power Measurement's Wheel Revolution Data field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelRevolutionData {
    /// Total wheel revolutions since the sensor was powered on. Wraps at
    /// `u32::MAX`.
    pub cumulative_revolutions: u32,
    /// Raw device timestamp, resolution 1/2048s (see
    /// [`crate::CP_WHEEL_EVENT_TIME_HZ`]). Wraps at `u16::MAX`; pair
    /// consecutive readings via [`crate::revolutions_per_minute`] rather
    /// than using this as an absolute clock.
    pub last_event_time_raw: u16,
}

/// Cumulative crank revolution count and the timestamp of the last one, as
/// reported by Cycling Power Measurement's Crank Revolution Data field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrankRevolutionData {
    /// Total crank revolutions since the sensor was powered on. Wraps at
    /// `u16::MAX`.
    pub cumulative_revolutions: u16,
    /// Raw device timestamp, resolution 1/1024s (see
    /// [`crate::CRANK_EVENT_TIME_HZ`]). Same wraparound caveat as
    /// [`WheelRevolutionData::last_event_time_raw`].
    pub last_event_time_raw: u16,
}

/// Cycling Power Measurement (characteristic 0x2A63).
#[derive(Debug, Clone, PartialEq)]
pub struct CyclingPowerMeasurement {
    /// Instantaneous power in watts. Signed per spec — regenerative
    /// trainers or a freewheeling/braking moment can report negative
    /// values.
    pub instantaneous_power_watts: i16,
    /// Left/right power split, if the power meter supports it.
    pub pedal_power_balance: Option<PedalPowerBalance>,
    /// Newton-metres, resolution 1/32 Nm.
    pub accumulated_torque_nm: Option<f32>,
    /// Wheel revolution count and timestamp, if the device reports speed
    /// via wheel rotation.
    pub wheel_revolutions: Option<WheelRevolutionData>,
    /// Crank revolution count and timestamp, if the device reports
    /// cadence.
    pub crank_revolutions: Option<CrankRevolutionData>,
    /// Maximum/minimum force in Newtons over the reporting period, if the
    /// device measures it.
    pub extreme_force_newtons: Option<MinMax<i16>>,
    /// Maximum/minimum torque in Newton-metres over the reporting period,
    /// if the device measures it.
    pub extreme_torque_nm: Option<MinMax<i16>>,
    /// Crank angle, in degrees, at which torque peaks (top dead spot).
    pub top_dead_spot_angle_deg: Option<u16>,
    /// Crank angle, in degrees, at which torque is at its minimum (bottom
    /// dead spot).
    pub bottom_dead_spot_angle_deg: Option<u16>,
    /// Cumulative energy expended, in kilojoules, since the last reset.
    pub accumulated_energy_kj: Option<u16>,
}

const PEDAL_POWER_BALANCE_PRESENT: u16 = 1 << 0;
const PEDAL_POWER_BALANCE_REFERENCE: u16 = 1 << 1;
const ACCUMULATED_TORQUE_PRESENT: u16 = 1 << 2;
const WHEEL_REV_PRESENT: u16 = 1 << 4;
const CRANK_REV_PRESENT: u16 = 1 << 5;
const EXTREME_FORCE_PRESENT: u16 = 1 << 6;
const EXTREME_TORQUE_PRESENT: u16 = 1 << 7;
const EXTREME_ANGLES_PRESENT: u16 = 1 << 8;
const TOP_DEAD_SPOT_PRESENT: u16 = 1 << 9;
const BOTTOM_DEAD_SPOT_PRESENT: u16 = 1 << 10;
const ACCUMULATED_ENERGY_PRESENT: u16 = 1 << 11;

/// Parses a raw Cycling Power Measurement (0x2A63) notification payload.
pub fn parse(data: &[u8]) -> Result<CyclingPowerMeasurement, ParseError> {
    let mut r = Reader::new(data);
    let flags = r.u16_le()?;

    // Spec defines Instantaneous Power as sint16, not uint16: regenerative
    // trainers or a freewheeling/braking moment can report negative watts.
    let instantaneous_power_watts = r.i16_le()?;

    let pedal_power_balance = if flags & PEDAL_POWER_BALANCE_PRESENT != 0 {
        let raw = r.u8()?;
        let reference = if flags & PEDAL_POWER_BALANCE_REFERENCE != 0 {
            PedalPowerBalanceReference::Left
        } else {
            PedalPowerBalanceReference::Unknown
        };
        Some(PedalPowerBalance {
            percent: raw as f32 / 2.0,
            reference,
        })
    } else {
        None
    };

    let accumulated_torque_nm = if flags & ACCUMULATED_TORQUE_PRESENT != 0 {
        Some(r.u16_le()? as f32 / 32.0)
    } else {
        None
    };

    let wheel_revolutions = if flags & WHEEL_REV_PRESENT != 0 {
        let cumulative_revolutions = r.u32_le()?;
        let last_event_time_raw = r.u16_le()?;
        Some(WheelRevolutionData {
            cumulative_revolutions,
            last_event_time_raw,
        })
    } else {
        None
    };

    let crank_revolutions = if flags & CRANK_REV_PRESENT != 0 {
        let cumulative_revolutions = r.u16_le()?;
        let last_event_time_raw = r.u16_le()?;
        Some(CrankRevolutionData {
            cumulative_revolutions,
            last_event_time_raw,
        })
    } else {
        None
    };

    // Spec defines both as sint16 (Newtons / Newton-metres) — can go
    // negative through part of the pedal stroke.
    let extreme_force_newtons = if flags & EXTREME_FORCE_PRESENT != 0 {
        let max = r.i16_le()?;
        let min = r.i16_le()?;
        Some(MinMax { max, min })
    } else {
        None
    };

    let extreme_torque_nm = if flags & EXTREME_TORQUE_PRESENT != 0 {
        let max = r.i16_le()?;
        let min = r.i16_le()?;
        Some(MinMax { max, min })
    } else {
        None
    };

    if flags & EXTREME_ANGLES_PRESENT != 0 {
        // Max/Min Angle packed as two 12-bit values across 3 bytes — not
        // decoded (no current consumer needs it, and it's rarely broadcast
        // by consumer power meters), but the bytes still have to be
        // consumed to keep every later field's offset correct.
        r.skip(3)?;
    }

    let top_dead_spot_angle_deg = if flags & TOP_DEAD_SPOT_PRESENT != 0 {
        Some(r.u16_le()?)
    } else {
        None
    };

    let bottom_dead_spot_angle_deg = if flags & BOTTOM_DEAD_SPOT_PRESENT != 0 {
        Some(r.u16_le()?)
    } else {
        None
    };

    let accumulated_energy_kj = if flags & ACCUMULATED_ENERGY_PRESENT != 0 {
        Some(r.u16_le()?)
    } else {
        None
    };

    Ok(CyclingPowerMeasurement {
        instantaneous_power_watts,
        pedal_power_balance,
        accumulated_torque_nm,
        wheel_revolutions,
        crank_revolutions,
        extreme_force_newtons,
        extreme_torque_nm,
        top_dead_spot_angle_deg,
        bottom_dead_spot_angle_deg,
        accumulated_energy_kj,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_power_only() {
        let data = [0x00, 0x00, 0xFA, 0x00]; // flags=0, power=250
        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_power_watts, 250);
        assert_eq!(m.pedal_power_balance, None);
    }

    #[test]
    fn negative_power() {
        let data = [0x00, 0x00, 0xFB, 0xFF]; // power = -5 as i16 LE
        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_power_watts, -5);
    }

    #[test]
    fn pedal_balance_with_left_reference() {
        let flags: u16 = 0x0003; // bit0 | bit1
        let data = [
            flags as u8,
            (flags >> 8) as u8,
            0xC8,
            0x00, // power = 200
            110,  // balance raw -> 55.0%
        ];
        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_power_watts, 200);
        let balance = m.pedal_power_balance.unwrap();
        assert_eq!(balance.percent, 55.0);
        assert_eq!(balance.reference, PedalPowerBalanceReference::Left);
    }

    #[test]
    fn pedal_balance_without_reference_is_unknown() {
        let flags: u16 = 0x0001; // bit0 only
        let data = [flags as u8, (flags >> 8) as u8, 0x96, 0x00, 100];
        let m = parse(&data).unwrap();
        let balance = m.pedal_power_balance.unwrap();
        assert_eq!(balance.percent, 50.0);
        assert_eq!(balance.reference, PedalPowerBalanceReference::Unknown);
    }

    #[test]
    fn crank_revolution_data() {
        let flags: u16 = 1 << 5;
        let data = [
            flags as u8,
            (flags >> 8) as u8,
            0xB4,
            0x00, // power = 180
            0xE8,
            0x03, // cumulative = 1000
            0x00,
            0x02, // last_event_time_raw = 512
        ];
        let m = parse(&data).unwrap();
        let crank = m.crank_revolutions.unwrap();
        assert_eq!(crank.cumulative_revolutions, 1000);
        assert_eq!(crank.last_event_time_raw, 512);
    }

    #[test]
    fn multiple_optional_fields_keep_correct_offsets() {
        // Exercises: pedal balance, wheel rev, crank rev, extreme angles
        // (skipped), accumulated energy — the field most likely to break if
        // any earlier optional field's byte width is wrong.
        let flags: u16 = (1 << 0) | (1 << 4) | (1 << 5) | (1 << 8) | (1 << 11);
        let mut data = vec![flags as u8, (flags >> 8) as u8];
        data.extend_from_slice(&100i16.to_le_bytes()); // power
        data.push(90); // pedal balance raw -> 45.0%, unknown reference
        data.extend_from_slice(&5000u32.to_le_bytes()); // wheel cumulative
        data.extend_from_slice(&30000u16.to_le_bytes()); // wheel last event time
        data.extend_from_slice(&800u16.to_le_bytes()); // crank cumulative
        data.extend_from_slice(&6000u16.to_le_bytes()); // crank last event time
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // extreme angles, skipped
        data.extend_from_slice(&250u16.to_le_bytes()); // accumulated energy

        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_power_watts, 100);
        assert_eq!(m.pedal_power_balance.unwrap().percent, 45.0);
        let wheel = m.wheel_revolutions.unwrap();
        assert_eq!(wheel.cumulative_revolutions, 5000);
        assert_eq!(wheel.last_event_time_raw, 30000);
        let crank = m.crank_revolutions.unwrap();
        assert_eq!(crank.cumulative_revolutions, 800);
        assert_eq!(crank.last_event_time_raw, 6000);
        assert_eq!(m.accumulated_energy_kj, Some(250));
    }

    #[test]
    fn too_short_errors() {
        assert!(parse(&[]).is_err());
        assert!(parse(&[0x00, 0x00]).is_err()); // flags but no power field
    }

    #[test]
    fn accumulated_torque() {
        let flags: u16 = 1 << 2; // torque present
        let mut data = vec![flags as u8, (flags >> 8) as u8];
        data.extend_from_slice(&150i16.to_le_bytes()); // power
        data.extend_from_slice(&320u16.to_le_bytes()); // raw 320 -> 10.0 Nm
        let m = parse(&data).unwrap();
        assert_eq!(m.accumulated_torque_nm, Some(10.0));
    }

    #[test]
    fn extreme_force_and_torque() {
        let flags: u16 = (1 << 6) | (1 << 7); // extreme force + extreme torque present
        let mut data = vec![flags as u8, (flags >> 8) as u8];
        data.extend_from_slice(&200i16.to_le_bytes()); // power
        data.extend_from_slice(&850i16.to_le_bytes()); // force max
        data.extend_from_slice(&(-120i16).to_le_bytes()); // force min
        data.extend_from_slice(&45i16.to_le_bytes()); // torque max
        data.extend_from_slice(&(-10i16).to_le_bytes()); // torque min
        let m = parse(&data).unwrap();
        let force = m.extreme_force_newtons.unwrap();
        assert_eq!(force.max, 850);
        assert_eq!(force.min, -120);
        let torque = m.extreme_torque_nm.unwrap();
        assert_eq!(torque.max, 45);
        assert_eq!(torque.min, -10);
    }

    #[test]
    fn top_and_bottom_dead_spot_angles() {
        let flags: u16 = (1 << 9) | (1 << 10); // top + bottom dead spot present
        let mut data = vec![flags as u8, (flags >> 8) as u8];
        data.extend_from_slice(&300i16.to_le_bytes()); // power
        data.extend_from_slice(&90u16.to_le_bytes());
        data.extend_from_slice(&270u16.to_le_bytes());
        let m = parse(&data).unwrap();
        assert_eq!(m.top_dead_spot_angle_deg, Some(90));
        assert_eq!(m.bottom_dead_spot_angle_deg, Some(270));
    }
}
