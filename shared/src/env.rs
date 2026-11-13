use core::{
    ffi::{CStr, c_char},
    str::Utf8Error,
};

#[link(name = "c")]
unsafe extern "C" {
    fn getenv(name: *const c_char) -> *const c_char;
}

pub fn var(name: &'static CStr) -> Result<Option<&'static str>, EnvVarError> {
    let value = unsafe { getenv(name.as_ptr()) };
    if value.is_null() {
        return Ok(None);
    }

    let value = unsafe { CStr::from_ptr(value) };
    let value = value.to_str().map_err(|err| EnvVarError { name, err })?;
    Ok(Some(value))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvVarError {
    name: &'static CStr,
    err: Utf8Error,
}

impl core::fmt::Display for EnvVarError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "env var {} must be valid utf-8: {}",
            self.name.to_str().unwrap_or("<non-utf8 name>"),
            self.err
        )
    }
}

impl core::error::Error for EnvVarError {}
