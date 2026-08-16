use crate::ParseError;

/// Little-endian byte cursor with bounds-checked reads. Every field-parsing
/// module in this crate goes through this instead of hand-rolled slicing —
/// GATT payloads are a sequence of optional fixed-width fields gated by flag
/// bits, and an off-by-one here silently misaligns every field after it.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        let end = self.pos + n;
        if end > self.data.len() {
            return Err(ParseError {
                needed: end,
                got: self.data.len(),
            });
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn i16_le(&mut self) -> Result<i16, ParseError> {
        let b = self.take(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    pub(crate) fn u16_le(&mut self) -> Result<u16, ParseError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    /// 3-byte unsigned little-endian integer (used by e.g. FTMS Total
    /// Distance) — no native Rust integer is this width, so widen into u32.
    pub(crate) fn u24_le(&mut self) -> Result<u32, ParseError> {
        let b = self.take(3)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], 0]))
    }

    pub(crate) fn u32_le(&mut self) -> Result<u32, ParseError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Advance past a field without decoding it (e.g. Cycling Power's
    /// Extreme Angles, whose 12-bit-packed layout isn't decoded yet) while
    /// keeping every later field's offset correct.
    pub(crate) fn skip(&mut self, n: usize) -> Result<(), ParseError> {
        self.take(n)?;
        Ok(())
    }

    pub(crate) fn remaining(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u8_reads_one_byte_and_advances() {
        let mut r = Reader::new(&[0x2A, 0xFF]);
        assert_eq!(r.u8().unwrap(), 0x2A);
        assert_eq!(r.u8().unwrap(), 0xFF);
    }

    #[test]
    fn i16_le_reads_little_endian_signed() {
        let mut r = Reader::new(&[0xFB, 0xFF]); // -5
        assert_eq!(r.i16_le().unwrap(), -5);
    }

    #[test]
    fn u16_le_reads_little_endian_unsigned() {
        let mut r = Reader::new(&[0xE8, 0x03]); // 1000
        assert_eq!(r.u16_le().unwrap(), 1000);
    }

    #[test]
    fn u24_le_reads_three_byte_little_endian() {
        // bytes [0x40, 0x42, 0x0F] -> 0x0F4240 -> 1_000_000
        let mut r = Reader::new(&[0x40, 0x42, 0x0F]);
        assert_eq!(r.u24_le().unwrap(), 1_000_000);
    }

    #[test]
    fn u24_le_max_value_does_not_bleed_into_fourth_byte() {
        let mut r = Reader::new(&[0xFF, 0xFF, 0xFF, 0x00]);
        assert_eq!(r.u24_le().unwrap(), 0x00FF_FFFF);
        // Confirms the 4th byte wasn't consumed by u24_le.
        assert_eq!(r.u8().unwrap(), 0x00);
    }

    #[test]
    fn u32_le_reads_little_endian_unsigned() {
        let mut r = Reader::new(&[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(r.u32_le().unwrap(), 0x1234_5678);
    }

    #[test]
    fn skip_advances_without_decoding() {
        let mut r = Reader::new(&[0xAA, 0xBB, 0xCC, 0x2A]);
        r.skip(3).unwrap();
        assert_eq!(r.u8().unwrap(), 0x2A);
    }

    #[test]
    fn remaining_reflects_current_position() {
        let mut r = Reader::new(&[1, 2, 3, 4]);
        assert_eq!(r.remaining(), &[1, 2, 3, 4]);
        r.u8().unwrap();
        assert_eq!(r.remaining(), &[2, 3, 4]);
    }

    #[test]
    fn read_past_end_errors_with_needed_and_got() {
        let mut r = Reader::new(&[0x01]);
        let err = r.u16_le().unwrap_err();
        assert_eq!(err.needed, 2);
        assert_eq!(err.got, 1);
    }

    #[test]
    fn error_reflects_position_after_prior_reads() {
        let mut r = Reader::new(&[0x01, 0x02, 0x03]);
        r.u16_le().unwrap(); // consumes 2 bytes
        let err = r.u16_le().unwrap_err(); // needs 2 more from pos=2 -> end=4
        assert_eq!(err.needed, 4);
        assert_eq!(err.got, 3);
    }
}
