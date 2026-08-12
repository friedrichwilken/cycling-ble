use crate::reader::Reader;
use crate::ParseError;

/// Heart Rate Measurement (characteristic 0x2A37).
#[derive(Debug, Clone, PartialEq)]
pub struct HeartRateMeasurement {
    /// Heart rate in beats per minute.
    pub bpm: u16,
    /// `None` when the sensor has no contact-detection feature at all.
    /// `Some(true)`/`Some(false)` when it does, reporting whether skin
    /// contact is currently detected. Kept as a tri-state on purpose: the
    /// spec packs "feature supported" and "contact detected" into two
    /// separate bits, and collapsing them into one bool (as some reference
    /// implementations do) makes "supported but not currently touching
    /// skin" indistinguishable from "no such feature" — the two matter
    /// differently for deciding whether to trust a reading.
    pub sensor_contact_detected: Option<bool>,
    /// Cumulative energy expended by the wearer since the last reset, in
    /// kilojoules, if the sensor reports it.
    pub energy_expended_kj: Option<u16>,
    /// RR intervals in seconds, converted from the spec's 1/1024s units.
    pub rr_intervals_secs: Vec<f32>,
}

const VALUE_FORMAT_UINT16: u8 = 1 << 0;
const SENSOR_CONTACT_FEATURE_SUPPORTED: u8 = 1 << 1;
const SENSOR_CONTACT_DETECTED: u8 = 1 << 2;
const ENERGY_EXPENDED_PRESENT: u8 = 1 << 3;
const RR_INTERVAL_PRESENT: u8 = 1 << 4;

/// Parses a raw Heart Rate Measurement (0x2A37) notification payload.
pub fn parse(data: &[u8]) -> Result<HeartRateMeasurement, ParseError> {
    let mut r = Reader::new(data);
    let flags = r.u8()?;

    let bpm = if flags & VALUE_FORMAT_UINT16 != 0 {
        r.u16_le()?
    } else {
        r.u8()? as u16
    };

    let sensor_contact_detected = if flags & SENSOR_CONTACT_FEATURE_SUPPORTED != 0 {
        Some(flags & SENSOR_CONTACT_DETECTED != 0)
    } else {
        None
    };

    let energy_expended_kj = if flags & ENERGY_EXPENDED_PRESENT != 0 {
        Some(r.u16_le()?)
    } else {
        None
    };

    let mut rr_intervals_secs = Vec::new();
    if flags & RR_INTERVAL_PRESENT != 0 {
        // Spec: "one or more" RR-Interval values fill the rest of the
        // packet — no count field, so read until fewer than 2 bytes remain.
        while r.remaining().len() >= 2 {
            let raw = r.u16_le()?;
            rr_intervals_secs.push(raw as f32 / 1024.0);
        }
    }

    Ok(HeartRateMeasurement {
        bpm,
        sensor_contact_detected,
        energy_expended_kj,
        rr_intervals_secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_uint8_bpm() {
        let data = [0x00, 65];
        let m = parse(&data).unwrap();
        assert_eq!(m.bpm, 65);
        assert_eq!(m.sensor_contact_detected, None);
        assert_eq!(m.energy_expended_kj, None);
        assert!(m.rr_intervals_secs.is_empty());
    }

    #[test]
    fn uint16_bpm_with_contact_and_energy() {
        // bit0 (u16 bpm) | bit1 (contact feature) | bit2 (contact detected) | bit3 (energy)
        let flags = 0b0000_1111;
        let data = [flags, 0x2C, 0x01, 0xF4, 0x01]; // bpm=300, energy=500
        let m = parse(&data).unwrap();
        assert_eq!(m.bpm, 300);
        assert_eq!(m.sensor_contact_detected, Some(true));
        assert_eq!(m.energy_expended_kj, Some(500));
    }

    #[test]
    fn contact_feature_supported_but_not_detected() {
        // The case a naive "either bit set" check gets wrong: feature
        // supported (bit1) but NOT currently detected (bit2 clear).
        let flags = 0b0000_0010;
        let data = [flags, 70];
        let m = parse(&data).unwrap();
        assert_eq!(m.sensor_contact_detected, Some(false));
    }

    #[test]
    fn rr_intervals() {
        let flags = 0b0001_0000;
        // 512 -> 0.5s, 1024 -> 1.0s
        let data = [flags, 80, 0x00, 0x02, 0x00, 0x04];
        let m = parse(&data).unwrap();
        assert_eq!(m.bpm, 80);
        assert_eq!(m.rr_intervals_secs, vec![0.5, 1.0]);
    }

    #[test]
    fn too_short_errors() {
        assert!(parse(&[]).is_err());
    }
}
