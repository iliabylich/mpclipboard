use crate::errors::ServerError;
use core::ffi::{CStr, c_char};

#[link(name = "c")]
unsafe extern "C" {
    fn getenv(name: *const c_char) -> *const c_char;
}

pub(crate) fn var(name: &'static CStr) -> Result<Option<&'static str>, ServerError> {
    let value = unsafe { getenv(name.as_ptr()) };
    if value.is_null() {
        return Ok(None);
    }

    let value = unsafe { CStr::from_ptr(value) };
    let value = value
        .to_str()
        .map_err(|err| ServerError::NonUtf8EnvVar { name, err })?;
    Ok(Some(value))
}
