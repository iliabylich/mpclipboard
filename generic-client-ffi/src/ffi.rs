use alloc::boxed::Box;
use core::{ffi::c_char, str::Utf8Error};
use mpclipboard_generic_client::{Connectivity, MPClipboard};
use rustix::fd::{AsFd, AsRawFd};

pub const MAX_TEXT_LENGTH: usize = 255;
const _: () = assert!(mpclipboard_shared::MAX_TEXT_LENGTH == MAX_TEXT_LENGTH);

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
pub extern "C" fn mpclipboard_new_from_file() -> Option<Box<MPClipboard>> {
    let mpclipboard = try_or_null!(MPClipboard::new_from_file());
    Some(Box::new(mpclipboard))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_get_fd(mpclipboard: &MPClipboard) -> i32 {
    mpclipboard.as_fd().as_raw_fd()
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_read(mpclipboard: &mut MPClipboard) -> Output {
    match mpclipboard.read() {
        Ok(output) => output.into(),
        Err(err) => {
            log::error!("error at FFI boundary: {err}");
            Output::ERROR
        }
    }
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

#[repr(C)]
pub struct BorrowedString {
    ptr: *const c_char,
    len: usize,
}

impl BorrowedString {
    pub(crate) const fn as_str(&self) -> Result<&str, Utf8Error> {
        let bytes = unsafe { core::slice::from_raw_parts(self.ptr.cast::<u8>(), self.len) };
        core::str::from_utf8(bytes)
    }
}

#[repr(C)]
pub struct Output {
    error: bool,
    has_connectivity: bool,
    connectivity: Connectivity,
    text_len: usize,
    text: [u8; MAX_TEXT_LENGTH],
}

impl Output {
    pub(crate) const ERROR: Self = Self {
        error: true,
        has_connectivity: false,
        connectivity: Connectivity::Disconnected,
        text_len: 0,
        text: [0; MAX_TEXT_LENGTH],
    };
}

impl From<mpclipboard_generic_client::Output> for Output {
    fn from(output: mpclipboard_generic_client::Output) -> Self {
        let (has_connectivity, connectivity) = match output.connectivity {
            Some(connectivity) => (true, connectivity),
            None => (false, Connectivity::Disconnected),
        };
        let mut text = [0; MAX_TEXT_LENGTH];
        let text_len = match output.text {
            Some(message) => {
                let bytes = message.text().as_bytes();
                text[..bytes.len()].copy_from_slice(bytes);
                bytes.len()
            }
            None => 0,
        };

        Self {
            error: false,
            has_connectivity,
            connectivity,
            text_len,
            text,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum PushResult {
    Pushed,
    Dropped,
    Error,
}
