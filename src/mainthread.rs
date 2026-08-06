// Getting work back onto the main thread. Scanning and cleaning both run off
// it, and every one of their results has to be applied where AppKit lives.
// Exports: `dispatch_to_main` and the trampolines `tasks` hands to it.
// Deps: libdispatch FFI, objc2, crate::{MenuHandler, HANDLER}.

use crate::{MenuHandler, HANDLER};
use objc2::msg_send;
use objc2::runtime::AnyObject;
use std::ffi::c_void;

pub(crate) fn dispatch_to_main(work: extern "C" fn(*mut c_void)) {
    unsafe {
        dispatch_async_f(std::ptr::addr_of!(_dispatch_main_q), std::ptr::null_mut(), work);
    }
}

macro_rules! trampoline {
    ($name:ident, $selector:ident) => {
        pub(crate) extern "C" fn $name(_ctx: *mut c_void) {
            HANDLER.with(|cell| {
                if let Some(handler) = cell.borrow().as_ref() {
                    let obj: &AnyObject =
                        unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
                    let _: () = unsafe { msg_send![obj, $selector: std::ptr::null::<AnyObject>()] };
                }
            });
        }
    };
}

trampoline!(scan_done_trampoline, scanDone);
trampoline!(sizes_tick_trampoline, sizesTick);
trampoline!(sizes_done_trampoline, sizesDone);
trampoline!(clean_done_trampoline, cleanDone);
trampoline!(progress_trampoline, cleanProgress);

#[link(name = "System", kind = "dylib")]
extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}
