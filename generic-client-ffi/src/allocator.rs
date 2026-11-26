use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
};

unsafe extern "C" {
    fn posix_memalign(memptr: *mut *mut c_void, alignment: usize, size: usize) -> i32;
    fn free(ptr: *mut c_void);
}

struct Malloc;

#[global_allocator]
static ALLOCATOR: Malloc = Malloc;

unsafe impl GlobalAlloc for Malloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let alignment = layout.align().max(size_of::<usize>());
        let mut ptr = core::ptr::null_mut();
        if unsafe { posix_memalign(&raw mut ptr, alignment, layout.size()) } != 0 {
            return core::ptr::null_mut();
        }
        ptr.cast()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { free(ptr.cast()) }
    }
}
