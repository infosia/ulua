//! Thread-local live Rust heap counts exclude other test workers and input construction.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct Counter;
thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static LIVE: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
}

fn add(bytes: usize) {
    if !ACTIVE.with(Cell::get) {
        return;
    }
    LIVE.with(|live| {
        let value = live.get() + bytes;
        live.set(value);
        PEAK.with(|peak| peak.set(peak.get().max(value)));
    });
}

// SAFETY: Each operation forwards the original pointer and layout to System.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller supplies a valid layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            add(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if ACTIVE.with(Cell::get) {
            LIVE.with(|live| live.set(live.get() - layout.size()));
        }
        // SAFETY: The caller supplies a live pointer and its original layout.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: The caller supplies the original allocation and a valid new size.
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            if size > layout.size() {
                add(size - layout.size());
            } else if ACTIVE.with(Cell::get) {
                LIVE.with(|live| live.set(live.get() - (layout.size() - size)));
            }
        }
        result
    }
}

#[global_allocator]
static ALLOCATOR: Counter = Counter;

pub fn measure<T>(compile: impl FnOnce() -> T) -> (T, usize) {
    // The synchronous compiler borrows inputs. It frees only allocations from this call.
    LIVE.with(|live| live.set(0));
    PEAK.with(|peak| peak.set(0));
    ACTIVE.with(|active| active.set(true));
    let result = compile();
    ACTIVE.with(|active| active.set(false));
    let peak = PEAK.with(Cell::get);
    (result, peak)
}
