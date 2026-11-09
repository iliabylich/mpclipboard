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
    match value.to_str() {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            log::error!(
                "env var {} must be valid utf-8: {err}",
                name.to_str().unwrap_or("<non-utf8 name>")
            );
            Err(())
        }
    }
}
