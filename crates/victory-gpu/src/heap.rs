//! The allocator of our own blocks. The system's process heap is one heap
//! for every thread: the submit lane, the eight recording threads and the
//! relay waited on its lock (avmpf3: 18% of the submit thread's samples and
//! 6-7% of each recording thread's sat inside it). mimalloc keeps a heap
//! per thread with no shared lock on the fast path. `VICTORY_MIMALLOC=0`
//! keeps the system's heap. The choice is made once, at the first
//! allocation (the main thread, before any other runs), and never changes,
//! so every block is released by the allocator that made it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

const UNKNOWN: u8 = 0;
const SYSTEM: u8 = 1;
const MIMALLOC: u8 = 2;

static CHOSEN: AtomicU8 = AtomicU8::new(UNKNOWN);

/// The allocator in use, read from the environment without allocating.
#[inline]
fn chosen() -> u8 {
    let now = CHOSEN.load(Relaxed);
    if now != UNKNOWN {
        return now;
    }
    let mut value = [0u8; 8];
    // SAFETY: a nul-terminated name and a buffer of the length given.
    let n = unsafe {
        windows_sys::Win32::System::Environment::GetEnvironmentVariableA(
            c"VICTORY_MIMALLOC".as_ptr().cast(),
            value.as_mut_ptr(),
            value.len() as u32,
        )
    };
    let pick = if n == 1 && value[0] == b'0' { SYSTEM } else { MIMALLOC };
    match CHOSEN.compare_exchange(UNKNOWN, pick, Relaxed, Relaxed) {
        Ok(_) => pick,
        Err(first) => first,
    }
}

/// Whether mimalloc holds our blocks (for the log).
pub fn is_mimalloc() -> bool {
    chosen() == MIMALLOC
}

/// mimalloc, or the system's heap when `VICTORY_MIMALLOC=0`.
pub struct Chosen;

// SAFETY: every call goes unchanged to one allocator, the same one for the
// whole process (see [`chosen`]).
unsafe impl GlobalAlloc for Chosen {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract, passed on.
        unsafe {
            if chosen() == SYSTEM { System.alloc(layout) } else { mimalloc::MiMalloc.alloc(layout) }
        }
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract, passed on.
        unsafe {
            if chosen() == SYSTEM { System.alloc_zeroed(layout) } else { mimalloc::MiMalloc.alloc_zeroed(layout) }
        }
    }

    #[inline]
    unsafe fn dealloc(&self, at: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract, passed on.
        unsafe {
            if chosen() == SYSTEM { System.dealloc(at, layout) } else { mimalloc::MiMalloc.dealloc(at, layout) }
        }
    }

    #[inline]
    unsafe fn realloc(&self, at: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: the caller's contract, passed on.
        unsafe {
            if chosen() == SYSTEM {
                System.realloc(at, layout, size)
            } else {
                mimalloc::MiMalloc.realloc(at, layout, size)
            }
        }
    }
}
