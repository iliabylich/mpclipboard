#![no_std]

extern crate alloc;

mod allocator;
mod ffi;

unsafe extern "C" {
    fn abort() -> !;
}

/// cbindgen:ignore
#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    log::error!("{info}");
    unsafe { abort() }
}
