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

#[repr(C)]
pub struct BorrowedString {
    ptr: *const c_char,
    len: usize,
}

impl BorrowedString {
    fn as_str(&self) -> Result<&str> {
        let bytes = unsafe { core::slice::from_raw_parts(self.ptr.cast::<u8>(), self.len) };
        core::str::from_utf8(bytes).context("non-utf8 string")
    }
}

#[repr(C)]
pub struct OwnedString {
    ptr: *mut c_char,
    len: usize,
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
pub enum COutput {
    ConnectivityChanged {
        connectivity: Connectivity,
    },
    NewText {
        text: OwnedString,
    },
    Both {
        connectivity: Connectivity,
        text: OwnedString,
    },
    Ignore,
    Error,
}
impl From<Output> for COutput {
    fn from(output: Output) -> Self {
        match (output.connectivity, output.text) {
            (Some(connectivity), None) => Self::ConnectivityChanged { connectivity },
            (None, Some(text)) => Self::NewText { text: text.into() },
            (Some(connectivity), Some(text)) => Self::Both {
                connectivity,
                text: text.into(),
            },
            (None, None) => Self::Ignore,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_read(mpclipboard: &mut MPClipboard) -> COutput {
    match mpclipboard.read() {
        Ok(output) => output.into(),
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
    text: BorrowedString,
) -> PushResult {
    let text = match text.as_str() {
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
pub extern "C" fn mpclipboard_drop_str(text: OwnedString) {
    drop(text);
}
