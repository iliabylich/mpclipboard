use crate::{Connectivity, MPClipboard};
use core::{ffi::c_char, str::Utf8Error};
use rustix::fd::AsRawFd;

macro_rules! try_or_null {
    ($v:expr) => {
        match $v {
            Ok(v) => v,
            Err(err) => {
                log::error!("error at FFI boundary: {err}");
                return None;
            }
        }
    };
}

#[repr(C)]
pub struct BorrowedString {
    ptr: *const c_char,
    len: usize,
}

impl BorrowedString {
    const fn as_str(&self) -> Result<&str, Utf8Error> {
        let bytes = unsafe { core::slice::from_raw_parts(self.ptr.cast::<u8>(), self.len) };
        core::str::from_utf8(bytes)
    }
}

#[repr(C)]
pub struct OwnedString {
    ptr: *mut c_char,
    len: usize,
}

impl OwnedString {
    const fn null() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            len: 0,
        }
    }
}

impl From<String> for OwnedString {
    fn from(s: String) -> Self {
        let s = s.into_boxed_str().into_boxed_bytes();
        let len = s.len();
        let ptr = Box::into_raw(s).cast::<c_char>();
        Self { ptr, len }
    }
}

impl Drop for OwnedString {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        let bytes: *mut [u8] = core::ptr::slice_from_raw_parts_mut(self.ptr.cast(), self.len);
        drop(unsafe { Box::from_raw(bytes) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_inline(
    url: BorrowedString,
    token: BorrowedString,
    id: BorrowedString,
) -> Option<Box<MPClipboard>> {
    let url = try_or_null!(url.as_str());
    let token = try_or_null!(token.as_str());
    let id = try_or_null!(id.as_str());

    let mpclipboard = try_or_null!(MPClipboard::new_inline(url, token, id));
    Some(Box::new(mpclipboard))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_with_local_config() -> Option<Box<MPClipboard>> {
    let mpclipboard = try_or_null!(MPClipboard::new_with_local_config());
    Some(Box::new(mpclipboard))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_with_xdg_config() -> Option<Box<MPClipboard>> {
    let mpclipboard = try_or_null!(MPClipboard::new_with_xdg_config());
    Some(Box::new(mpclipboard))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_get_fd(mpclipboard: &MPClipboard) -> i32 {
    mpclipboard.as_raw_fd()
}

#[repr(C)]
pub struct Output {
    error: bool,
    has_connectivity: bool,
    connectivity: Connectivity,
    text: OwnedString,
}

impl Output {
    const fn error() -> Self {
        Self {
            error: true,
            has_connectivity: false,
            connectivity: Connectivity::Disconnected,
            text: OwnedString::null(),
        }
    }
}

impl From<crate::Output> for Output {
    fn from(output: crate::Output) -> Self {
        let (has_connectivity, connectivity) = match output.connectivity {
            Some(connectivity) => (true, connectivity),
            None => (false, Connectivity::Disconnected),
        };
        let text = match output.text {
            Some(text) => OwnedString::from(text),
            None => OwnedString::null(),
        };

        Self {
            error: false,
            has_connectivity,
            connectivity,
            text,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_read(mpclipboard: &mut MPClipboard) -> Output {
    match mpclipboard.read() {
        Ok(output) => output.into(),
        Err(err) => {
            log::error!("error at FFI boundary: {err}");
            Output::error()
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum PushResult {
    Pushed,
    Dropped,
    Error,
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_push_text(
    mpclipboard: &mut MPClipboard,
    text: BorrowedString,
) -> PushResult {
    let text = match text.as_str() {
        Ok(text) => text,
        Err(err) => {
            log::error!("error at FFI boundary: {err}");
            return PushResult::Error;
        }
    };

    match mpclipboard.push_text(text) {
        Ok(true) => PushResult::Pushed,
        Ok(false) => PushResult::Dropped,
        Err(err) => {
            log::error!("error at FFI boundary: {err}");
            PushResult::Error
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_drop(mpclipboard: Box<MPClipboard>) {
    drop(mpclipboard);
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_drop_str(text: OwnedString) {
    drop(text);
}
