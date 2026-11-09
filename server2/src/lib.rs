#![no_std]

mod stderr;
pub use stderr::Stderr;

use externally_driven_async_rt::Selector;

pub fn run() -> i32 {
    match Selector::new() {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_run() {
        assert_eq!(super::run(), 0);
    }
}
