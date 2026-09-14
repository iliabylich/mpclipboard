use anyhow::{Context, Result, anyhow};
use boml::Toml;
use rustix::fs::{Mode, OFlags};

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<const N: usize, T>(
        path: &[u8],
        buffer: &mut [u8],
        keys: [&'static str; N],
        f: impl FnOnce([&str; N]) -> T,
    ) -> Result<T> {
        let toml = read_toml(path, buffer)?;

        let mut values = [""; N];

        for (key, slot) in keys.iter().zip(values.iter_mut()) {
            let value = toml
                .get_string(key)
                .map_err(|err| anyhow!("key {key} must be a string in toml: {err:?}"))?;

            *slot = value;
        }

        Ok(f(values))
    }
}

fn read_toml<'a>(path: &[u8], buffer: &'a mut [u8]) -> Result<Toml<'a>> {
    let fd =
        rustix::fs::open(path, OFlags::RDONLY, Mode::empty()).context("failed to open() config")?;
    let len = rustix::io::read(&fd, &mut *buffer).context("failed to read() config")?;
    let bytes = buffer
        .get(..len)
        .unwrap_or_else(|| unreachable!("read() returned malformed data"));
    let text = str::from_utf8(bytes).context("config must be valid utf-8")?;

    boml::parse(text).map_err(|err| anyhow!("failed to parse TOML config: {err:}"))
}
