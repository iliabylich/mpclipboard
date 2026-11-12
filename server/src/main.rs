#![no_std]
#![no_main]

use core::fmt::Write as _;

unsafe extern "C" {
    fn abort() -> !;
}

#[unsafe(no_mangle)]
extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let Err(err) = server2::run();
    let _ = writeln!(server2::Stderr, "ERROR {err}");
    1
}

#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    let _ = writeln!(server2::Stderr, "{info}");
    unsafe { abort() }
}
