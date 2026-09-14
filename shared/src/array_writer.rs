pub(crate) struct ArrayWriter<'a> {
    buf: &'a mut [u8],
    offset: usize,
}

impl<'a> ArrayWriter<'a> {
    pub(crate) const fn new(buf: &'a mut [u8]) -> Self {
        ArrayWriter { buf, offset: 0 }
    }

    pub(crate) const fn as_bytes(&self) -> &[u8] {
        let (head, _tail) = self.buf.split_at(self.offset);
        head
    }
}

impl core::fmt::Write for ArrayWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();

        let remainder = self.buf.get_mut(self.offset..).ok_or(core::fmt::Error)?;
        if remainder.len() < bytes.len() {
            return Err(core::fmt::Error);
        }
        let remainder = remainder.get_mut(..bytes.len()).ok_or(core::fmt::Error)?;
        remainder.copy_from_slice(bytes);

        self.offset = self
            .offset
            .checked_add(bytes.len())
            .ok_or(core::fmt::Error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ArrayWriter;
    use anyhow::Result;
    use core::fmt::Write;

    #[test]
    fn test_write_ok() -> Result<()> {
        let mut buf = [0; 5];
        let mut writer = ArrayWriter::new(&mut buf);
        write!(&mut writer, "1{}3{}5", 2, 4)?;
        assert_eq!(writer.as_bytes(), b"12345");
        Ok(())
    }

    #[test]
    fn test_write_overflow() {
        let mut buf = [0; 2];
        let mut writer = ArrayWriter::new(&mut buf);
        assert_eq!(write!(&mut writer, "1{}3", 2), Err(core::fmt::Error));
    }
}
