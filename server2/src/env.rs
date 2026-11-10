use core::ffi::{CStr, c_char};

#[link(name = "c")]
unsafe extern "C" {
    fn getenv(name: *const c_char) -> *const c_char;
}

pub(crate) fn var(name: &CStr) -> Result<Option<&'static str>, ()> {
    let value = unsafe { getenv(name.as_ptr()) };
    if value.is_null() {
        return Ok(None);
    }

    let value = unsafe { CStr::from_ptr(value) };
    let value = value
        .to_str()
        .inspect_err(|err| {
            log::error!(
                "env var {} must be valid utf-8: {err}",
                name.to_str().unwrap_or("<non-utf8 name>")
            );
        })
        .map_err(|_| ())?;
    Ok(Some(value))
}
