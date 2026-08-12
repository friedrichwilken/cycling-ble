use crate::reader::Reader;
use crate::ParseError;

/// Indoor Bike Data (characteristic 0x2AD2, part of the Fitness Machine
/// Service) — the combined speed/cadence/power/HR stream many smart
/// trainers broadcast instead of (or alongside) the separate Cycling Power
/// and CSC services.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IndoorBikeData {
    /// km/h
    pub instantaneous_speed: Option<f32>,
    /// km/h, averaged over the elapsed workout so far.
    pub average_speed: Option<f32>,
    /// rpm
    pub instantaneous_cadence: Option<f32>,
    /// rpm, averaged over the elapsed workout so far.
    pub average_cadence: Option<f32>,
    /// metres
    pub total_distance: Option<u32>,
    /// Unitless resistance-level setting reported by the trainer.
    pub resistance_level: Option<i16>,
    /// watts
    pub instantaneous_power: Option<i16>,
    /// watts, averaged over the elapsed workout so far.
    pub average_power: Option<i16>,
    /// kcal
    pub total_energy: Option<u16>,
    /// kcal/h, current expenditure rate.
    pub energy_per_hour: Option<u16>,
    /// kcal/min, current expenditure rate.
    pub energy_per_minute: Option<u8>,
    /// Heart rate in bpm, if the trainer has its own HR input.
    pub heart_rate_bpm: Option<u8>,
    /// Metabolic equivalent of task (METs).
    pub metabolic_equivalent: Option<f32>,
    /// Seconds elapsed in the current workout.
    pub elapsed_time_secs: Option<u16>,
    /// Seconds remaining in the current workout, if the trainer knows its
    /// target duration.
    pub remaining_time_secs: Option<u16>,
}

// Note the inversion on bit 0: every other flag in this field follows
// "1 = field present," but bit 0 ("More Data") is the opposite — 0 means
// Instantaneous Speed IS present, 1 means it's absent. This really is how
// the Bluetooth SIG spec defines it, confirmed against a real-device-tested
// reference implementation (some vendor docs describe it without the
// inversion, which turns out not to match real trainers) — easy to get
// backwards if you pattern-match the other bits' convention.
const MORE_DATA: u16 = 1 << 0;
const AVERAGE_SPEED_PRESENT: u16 = 1 << 1;
const INSTANTANEOUS_CADENCE_PRESENT: u16 = 1 << 2;
const AVERAGE_CADENCE_PRESENT: u16 = 1 << 3;
const TOTAL_DISTANCE_PRESENT: u16 = 1 << 4;
const RESISTANCE_LEVEL_PRESENT: u16 = 1 << 5;
const INSTANTANEOUS_POWER_PRESENT: u16 = 1 << 6;
const AVERAGE_POWER_PRESENT: u16 = 1 << 7;
const EXPENDED_ENERGY_PRESENT: u16 = 1 << 8;
const HEART_RATE_PRESENT: u16 = 1 << 9;
const METABOLIC_EQUIVALENT_PRESENT: u16 = 1 << 10;
const ELAPSED_TIME_PRESENT: u16 = 1 << 11;
const REMAINING_TIME_PRESENT: u16 = 1 << 12;

/// Parses a raw Indoor Bike Data (0x2AD2) notification payload.
pub fn parse(data: &[u8]) -> Result<IndoorBikeData, ParseError> {
    let mut r = Reader::new(data);
    let flags = r.u16_le()?;
    let mut out = IndoorBikeData::default();

    if flags & MORE_DATA == 0 {
        out.instantaneous_speed = Some(r.u16_le()? as f32 / 100.0);
    }
    if flags & AVERAGE_SPEED_PRESENT != 0 {
        out.average_speed = Some(r.u16_le()? as f32 / 100.0);
    }
    if flags & INSTANTANEOUS_CADENCE_PRESENT != 0 {
        out.instantaneous_cadence = Some(r.u16_le()? as f32 / 2.0);
    }
    if flags & AVERAGE_CADENCE_PRESENT != 0 {
        out.average_cadence = Some(r.u16_le()? as f32 / 2.0);
    }
    if flags & TOTAL_DISTANCE_PRESENT != 0 {
        out.total_distance = Some(r.u24_le()?);
    }
    if flags & RESISTANCE_LEVEL_PRESENT != 0 {
        out.resistance_level = Some(r.i16_le()?);
    }
    if flags & INSTANTANEOUS_POWER_PRESENT != 0 {
        out.instantaneous_power = Some(r.i16_le()?);
    }
    if flags & AVERAGE_POWER_PRESENT != 0 {
        out.average_power = Some(r.i16_le()?);
    }
    if flags & EXPENDED_ENERGY_PRESENT != 0 {
        out.total_energy = Some(r.u16_le()?);
        out.energy_per_hour = Some(r.u16_le()?);
        out.energy_per_minute = Some(r.u8()?);
    }
    if flags & HEART_RATE_PRESENT != 0 {
        out.heart_rate_bpm = Some(r.u8()?);
    }
    if flags & METABOLIC_EQUIVALENT_PRESENT != 0 {
        out.metabolic_equivalent = Some(r.u8()? as f32 / 10.0);
    }
    if flags & ELAPSED_TIME_PRESENT != 0 {
        out.elapsed_time_secs = Some(r.u16_le()?);
    }
    if flags & REMAINING_TIME_PRESENT != 0 {
        out.remaining_time_secs = Some(r.u16_le()?);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn more_data_clear_means_speed_present() {
        let data = [0x00, 0x00, 0xC4, 0x09]; // flags=0, speed raw 2500 -> 25.00 km/h
        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_speed, Some(25.0));
    }

    #[test]
    fn more_data_set_means_speed_absent() {
        let flags: u16 = (1 << 0) | (1 << 6); // more-data set (speed absent) + power present
        let data = [flags as u8, (flags >> 8) as u8, 0xFA, 0x00]; // power=250
        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_speed, None);
        assert_eq!(m.instantaneous_power, Some(250));
    }

    #[test]
    fn cadence_half_rpm_resolution() {
        let flags: u16 = (1 << 0) | (1 << 2); // speed absent, cadence present
        let data = [flags as u8, (flags >> 8) as u8, 0xAA, 0x00]; // raw 170 -> 85.0 rpm
        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_cadence, Some(85.0));
    }

    #[test]
    fn multiple_fields_keep_correct_offsets() {
        let flags: u16 = (1 << 0) | (1 << 2) | (1 << 6) | (1 << 9) | (1 << 11);
        let mut data = vec![flags as u8, (flags >> 8) as u8];
        data.extend_from_slice(&180u16.to_le_bytes()); // cadence raw -> 90.0 rpm
        data.extend_from_slice(&300i16.to_le_bytes()); // power
        data.push(145); // heart rate
        data.extend_from_slice(&3600u16.to_le_bytes()); // elapsed time

        let m = parse(&data).unwrap();
        assert_eq!(m.instantaneous_speed, None);
        assert_eq!(m.instantaneous_cadence, Some(90.0));
        assert_eq!(m.instantaneous_power, Some(300));
        assert_eq!(m.heart_rate_bpm, Some(145));
        assert_eq!(m.elapsed_time_secs, Some(3600));
    }

    #[test]
    fn too_short_errors() {
        assert!(parse(&[]).is_err());
    }
}
