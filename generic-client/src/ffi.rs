use crate::{Connectivity, MPClipboard, Output};
use anyhow::{Context, Result};
use std::{ffi::c_char, os::fd::AsRawFd};

macro_rules! try_or_null {
    ($v:expr) => {
        match $v {
            Ok(v) => v,
            Err(err) => {
                log::error!("error at FFI boundary: {err:?}");
                return None;
            }
        }
    };
}

fn bytes_to_str<'a>(ptr: *const c_char, len: usize) -> Result<&'a str> {
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    core::str::from_utf8(bytes).context("non-utf8 string")
}
fn string_to_c(s: String) -> (*mut c_char, usize) {
    let s = s.into_boxed_str().into_boxed_bytes();
    let len = s.len();
    let ptr = Box::into_raw(s).cast::<c_char>();
    (ptr, len)
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new_inline(
    url_ptr: *const c_char,
    url_len: usize,
    token_ptr: *const c_char,
    token_len: usize,
    id_ptr: *const c_char,
    id_len: usize,
) -> Option<Box<MPClipboard>> {
    let url = try_or_null!(bytes_to_str(url_ptr, url_len));
    let token = try_or_null!(bytes_to_str(token_ptr, token_len));
    let id = try_or_null!(bytes_to_str(id_ptr, id_len));

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
pub extern "C" fn mpclipboard_read(mpclipboard: &mut MPClipboard) -> COutput {
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
    mpclipboard: &mut MPClipboard,
    ptr: *const c_char,
    len: usize,
) -> PushResult {
    let text = match bytes_to_str(ptr, len) {
        Ok(text) => text,
        Err(err) => {
            log::error!("error at FFI boundary: {err:?}");
            return PushResult::Error;
        }
    };

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
pub extern "C" fn mpclipboard_drop(mpclipboard: Box<MPClipboard>) {
    drop(mpclipboard);
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_drop_str(ptr: *mut c_char, len: usize) {
    let bytes: *mut [u8] = std::ptr::slice_from_raw_parts_mut(ptr.cast(), len);
    let boxed: Box<[u8]> = unsafe { Box::from_raw(bytes) };
    drop(boxed);
}
