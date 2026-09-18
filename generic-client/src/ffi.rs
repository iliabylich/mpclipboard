use crate::{Connectivity, MPClipboard, Output};
use anyhow::{Context, Result};
use std::{ffi::c_char, os::fd::AsRawFd};

macro_rules! try_or_null {
    ($v:expr) => {
        match $v {
            Ok(v) => v,
            Err(err) => {
                log::error!("error at FFI boundary: {err:?}");
                return core::ptr::null_mut();
            }
        }
    };
}

fn cstring_to_str(s: *const c_char) -> Result<&'static str> {
    let s = unsafe { std::ffi::CStr::from_ptr(s) };
    s.to_str().context("non-utf8 string")
}
fn string_to_c(s: String) -> (*mut c_char, usize) {
    let s = s.into_boxed_str().into_boxed_bytes();
    let len = s.len();
    let ptr = Box::into_raw(s).cast::<c_char>();
    (ptr, len)
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_inline(
    url: *const c_char,
    token: *const c_char,
    id: *const c_char,
) -> *mut MPClipboard {
    let url = try_or_null!(cstring_to_str(url));
    let token = try_or_null!(cstring_to_str(token));
    let id = try_or_null!(cstring_to_str(id));

    let mpclipboard = try_or_null!(MPClipboard::new_inline(url, token, id));
    Box::leak(Box::new(mpclipboard))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_with_local_config() -> *mut MPClipboard {
    let mpclipboard = try_or_null!(MPClipboard::new_with_local_config());
    Box::leak(Box::new(mpclipboard))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_with_xdg_config() -> *mut MPClipboard {
    let mpclipboard = try_or_null!(MPClipboard::new_with_xdg_config());
    Box::leak(Box::new(mpclipboard))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_get_fd(mpclipboard: *mut MPClipboard) -> i32 {
    let mpclipboard = unsafe { &*mpclipboard };
    mpclipboard.as_raw_fd()
}

#[repr(C)]
pub enum COutput {
    ConnectivityChanged {
        connectivity: Connectivity,
    },
    NewText {
        ptr: *mut c_char,
        len: usize,
    },
    Both {
        connectivity: Connectivity,
        ptr: *mut c_char,
        len: usize,
    },
    Ignore,
    Error,
}
impl From<Output> for COutput {
    fn from(output: Output) -> Self {
        match output {
            Output::ConnectivityChanged { connectivity } => {
                Self::ConnectivityChanged { connectivity }
            }
            Output::NewText { text } => {
                let (ptr, len) = string_to_c(text);
                Self::NewText { ptr, len }
            }
            Output::Both { connectivity, text } => {
                let (ptr, len) = string_to_c(text);
                Self::Both {
                    connectivity,
                    ptr,
                    len,
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_read(mpclipboard: *mut MPClipboard) -> COutput {
    let mpclipboard = unsafe { &mut *mpclipboard };
    match mpclipboard.read() {
        Ok(Some(output)) => output.into(),
        Ok(None) => COutput::Ignore,
        Err(err) => {
            log::error!("error at FFI boundary: {err:?}");
            COutput::Error
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
    mpclipboard: *mut MPClipboard,
    ptr: *const c_char,
    len: usize,
) -> PushResult {
    let mpclipboard = unsafe { &mut *mpclipboard };
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    let text = unsafe { std::str::from_utf8_unchecked(bytes) };

    match mpclipboard.push_text(text) {
        Ok(true) => PushResult::Pushed,
        Ok(false) => PushResult::Dropped,
        Err(err) => {
            log::error!("error at FFI boundary: {err:?}");
            PushResult::Error
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_drop(mpclipboard: *mut MPClipboard) {
    unsafe { core::ptr::drop_in_place(mpclipboard) };
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_drop_str(ptr: *mut c_char, len: usize) {
    let bytes: *mut [u8] = std::ptr::slice_from_raw_parts_mut(ptr.cast(), len);
    let boxed: Box<[u8]> = unsafe { Box::from_raw(bytes) };
    drop(boxed);
}
