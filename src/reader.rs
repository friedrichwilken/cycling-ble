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
