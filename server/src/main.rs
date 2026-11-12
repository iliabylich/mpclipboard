#![no_std]
#![no_main]

use core::fmt::Write as _;

unsafe extern "C" {
    fn abort() -> !;
}

#[unsafe(no_mangle)]
extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let Err(err) = mpclipboard_server::run();
    let _ = writeln!(mpclipboard_server::Stderr, "ERROR {err}");
    1
}

#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    let _ = writeln!(mpclipboard_server::Stderr, "{info}");
    unsafe { abort() }
}
