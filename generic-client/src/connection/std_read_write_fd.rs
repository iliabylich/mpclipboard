use std::{
    io::{Read, Write},
    os::fd::AsFd,
};

pub struct StdReadWriteFd<'a, F>(pub &'a F);

impl<F> Read for StdReadWriteFd<'_, F>
where
    F: AsFd,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = rustix::io::read(self.0, buf)?;
        Ok(len)
    }
}

impl<F> Write for StdReadWriteFd<'_, F>
where
    F: AsFd,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = rustix::io::write(self.0, buf)?;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
