use crate::reader::Reader;
use crate::ParseError;

/// Cumulative wheel revolution count and the timestamp of the last one, as
/// reported by CSC Measurement's Wheel Revolution Data field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelRevolutionData {
    /// Total wheel revolutions since the sensor was powered on. Wraps at
    /// `u32::MAX`.
    pub cumulative_revolutions: u32,
    /// Raw device timestamp, resolution 1/1024s (see
    /// [`crate::CSC_WHEEL_EVENT_TIME_HZ`]) — distinct from Cycling Power's
    /// wheel-event-time resolution of 1/2048s. Same field name, different
    /// unit, because it's a different GATT service with its own spec.
    pub last_event_time_raw: u16,
}

/// Cumulative crank revolution count and the timestamp of the last one, as
/// reported by CSC Measurement's Crank Revolution Data field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrankRevolutionData {
    /// Total crank revolutions since the sensor was powered on. Wraps at
    /// `u16::MAX`.
    pub cumulative_revolutions: u16,
    /// Raw device timestamp, resolution 1/1024s (see
    /// [`crate::CRANK_EVENT_TIME_HZ`]).
    pub last_event_time_raw: u16,
}

/// CSC (Cycling Speed and Cadence) Measurement (characteristic 0x2A5B).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CscMeasurement {
    /// Present when the sensor supports wheel-revolution reporting (e.g. a
    /// speed sensor).
    pub wheel_revolutions: Option<WheelRevolutionData>,
    /// Present when the sensor supports crank-revolution reporting (e.g. a
    /// cadence sensor).
    pub crank_revolutions: Option<CrankRevolutionData>,
}

const WHEEL_REV_PRESENT: u8 = 1 << 0;
const CRANK_REV_PRESENT: u8 = 1 << 1;

/// Parses a raw CSC Measurement (0x2A5B) notification payload.
pub fn parse(data: &[u8]) -> Result<CscMeasurement, ParseError> {
    let mut r = Reader::new(data);
    let flags = r.u8()?;

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

    Ok(CscMeasurement {
        wheel_revolutions,
        crank_revolutions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_flags_set() {
        let m = parse(&[0x00]).unwrap();
        assert_eq!(m.wheel_revolutions, None);
        assert_eq!(m.crank_revolutions, None);
    }

    #[test]
    fn wheel_then_crank_offsets() {
        let flags = 0b0000_0011u8; // both present
        let mut data = vec![flags];
        data.extend_from_slice(&12345u32.to_le_bytes());
        data.extend_from_slice(&2000u16.to_le_bytes());
        data.extend_from_slice(&600u16.to_le_bytes());
        data.extend_from_slice(&4000u16.to_le_bytes());

        let m = parse(&data).unwrap();
        let wheel = m.wheel_revolutions.unwrap();
        assert_eq!(wheel.cumulative_revolutions, 12345);
        assert_eq!(wheel.last_event_time_raw, 2000);
        let crank = m.crank_revolutions.unwrap();
        assert_eq!(crank.cumulative_revolutions, 600);
        assert_eq!(crank.last_event_time_raw, 4000);
    }

    #[test]
    fn too_short_errors() {
        assert!(parse(&[]).is_err());
        assert!(parse(&[0b0000_0010]).is_err()); // crank flag set, no data
    }
}
