use rustix::io::Errno;

pub struct Stderr;

impl core::fmt::Write for Stderr {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let fd = unsafe { rustix::stdio::stderr() };
        let mut buf = s.as_bytes();
        while !buf.is_empty() {
            match rustix::io::write(fd, buf) {
                Ok(0) => return Err(core::fmt::Error),
                Ok(n) => buf = &buf[n..],
                Err(Errno::INTR) => {}
                Err(_) => return Err(core::fmt::Error),
            }
        }
        Ok(())
    }
}
