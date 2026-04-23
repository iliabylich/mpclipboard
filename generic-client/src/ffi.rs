use crate::{Config, ConfigReadOption, Connectivity, Context, MPClipboard, Output};
use std::{ffi::c_char, os::fd::AsRawFd};

macro_rules! try_or_null {
    ($v:expr) => {
        match $v {
            Ok(v) => v,
            Err(err) => {
                log::error!("{err:?}");
                return core::ptr::null_mut();
            }
        }
    };
}

fn cstring_to_string(s: *const c_char) -> String {
    unsafe { std::ffi::CStr::from_ptr(s) }
        .to_string_lossy()
        .to_string()
}
fn string_to_c(s: String) -> (*mut c_char, usize) {
    let (ptr, len, _capacity) = s.into_raw_parts();
    (ptr.cast(), len)
}

/// Initializes MPClipboard, must be called once at startup
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_init() -> bool {
    if let Err(err) = MPClipboard::init() {
        log::error!("{err:?}");
        false
    } else {
        true
    }
}

/// Reads the config based on the given instruction
/// (which is either "read from XDG dir" or "read from local ./config.toml").
/// In case of an error logs it and returns NULL.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_config_read(option: ConfigReadOption) -> *mut Config {
    let config = try_or_null!(Config::read(option));
    Box::leak(Box::new(config))
}

/// Constructs the config in-place based on given parameters that match fields 1-to-1.
/// In case of an error logs it and returns NULL.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_config_new(
    uri: *const c_char,
    token: *const c_char,
    name: *const c_char,
) -> *mut Config {
    let uri = cstring_to_string(uri);
    let token = cstring_to_string(token);
    let name = cstring_to_string(name);
    let config = try_or_null!(Config::new(uri, token, name));
    Box::leak(Box::new(config))
}

/// Constructs a new MPClipboard context.
/// Consumes config.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_context_new(config: *mut Config) -> *mut Context {
    let config = unsafe { *Box::from_raw(config) };
    let context = try_or_null!(Context::new(config));
    Box::leak(Box::new(context))
}

/// Constructs a new MPClipboard.
/// Consumes context.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_new(context: *mut Context) -> *mut MPClipboard {
    let context = unsafe { *Box::from_raw(context) };
    Box::leak(Box::new(MPClipboard::new(context)))
}

/// Constructs a new MPClipboard.
/// Consumes context.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_get_fd(mpclipboard: *mut MPClipboard) -> i32 {
    let mpclipboard = unsafe { &mut *mpclipboard };
    mpclipboard.as_raw_fd()
}

/// Result of reading
#[repr(C)]
pub enum COutput {
    /// An event indicating that connectivity changed, guaranteed to be different from a previous one
    ConnectivityChanged {
        /// New connecivity
        connectivity: Connectivity,
    },
    /// New text clip
    NewText {
        /// New text
        ptr: *mut c_char,
        /// and its length
        len: usize,
    },
    /// Internal
    Internal,
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
        }
    }
}

/// Reads from a given MPClipboard instance.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_read(mpclipboard: *mut MPClipboard) -> COutput {
    let mpclipboard = unsafe { &mut *mpclipboard };
    match mpclipboard.read() {
        Some(output) => output.into(),
        None => COutput::Internal,
    }
}

/// Pushes text from NULL-terminated C-style string,
/// returns false if given text isn't new
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_push_text1(
    mpclipboard: *mut MPClipboard,
    text: *const c_char,
) -> bool {
    let mpclipboard = unsafe { &mut *mpclipboard };
    let text = cstring_to_string(text);
    mpclipboard.push_text(text)
}

/// Pushes text from pointer + length
/// returns false if given text isn't new
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_push_text2(
    mpclipboard: *mut MPClipboard,
    ptr: *const c_char,
    len: usize,
) -> bool {
    let mpclipboard = unsafe { &mut *mpclipboard };
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    let text = unsafe { std::str::from_utf8_unchecked(bytes) };
    mpclipboard.push_text(text.to_string())
}

/// Drops an instance of MPClipboard, frees memory, closes files
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_drop(mpclipboard: *mut MPClipboard) {
    unsafe { core::ptr::drop_in_place(mpclipboard) };
}

/// Prints one "info" and one "error" message, useful for testing
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_logger_test() {
    log::info!("info example");
    log::error!("error example");
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_setup_rustls_on_jvm(
    env: *mut jni::sys::JNIEnv,
    context: jni::sys::jobject,
) {
    let mut env = match unsafe { jni::JNIEnv::from_raw(env) } {
        Ok(env) => env,
        Err(err) => {
            log::error!("JNIEnv::from_raw failed: {:?}", err);
            return;
        }
    };
    let context = unsafe { jni::objects::JObject::from_raw(context) };

    if let Err(err) = rustls_platform_verifier::android::init_hosted(&mut env, context) {
        log::error!("Failed to instantiate rustls_platform_verifier: {err:?}");
    }
}
