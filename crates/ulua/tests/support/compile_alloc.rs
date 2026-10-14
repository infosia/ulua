//! Live Rust heap counts cover the ABI caller and its compiler worker.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
};

struct Counter;
#[derive(Clone, Copy)]
struct Header {
    epoch: usize,
}
static EPOCH: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
thread_local! {
    static CALLER: Cell<bool> = const { Cell::new(false) };
    static INSPECT: Cell<bool> = const { Cell::new(false) };
    static WORKER: Cell<Option<bool>> = const { Cell::new(None) };
}

fn epoch() -> usize {
    let epoch = ACTIVE.load(SeqCst);
    if epoch == 0 {
        return 0;
    }
    if CALLER.with(Cell::get) {
        return epoch;
    }
    if let Some(worker) = WORKER.with(Cell::get) {
        return if worker { epoch } else { 0 };
    }
    // Thread metadata can allocate on first access. Exclude those recursive allocations.
    INSPECT.with(|inspect| {
        if inspect.replace(true) {
            return 0;
        }
        let worker = std::thread::current().name() == Some("ulua-compiler");
        WORKER.with(|cached| cached.set(Some(worker)));
        inspect.set(false);
        if worker { epoch } else { 0 }
    })
}

fn storage(layout: Layout) -> Option<(Layout, usize)> {
    // Header size and both alignments are powers of two. This offset aligns the payload.
    let offset = size_of::<Header>().max(layout.align());
    let size = layout.size().checked_add(offset)?;
    Layout::from_size_align(size, align_of::<Header>().max(layout.align()))
        .ok()
        .map(|storage| (storage, offset))
}

// SAFETY: Allocation metadata precedes the aligned payload. System owns each backing allocation.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some((storage, offset)) = storage(layout) else {
            return std::ptr::null_mut();
        };
        // SAFETY: Storage includes the header and the aligned payload.
        let pointer = unsafe { System.alloc(storage) };
        if pointer.is_null() {
            return pointer;
        }
        let epoch = epoch();
        // SAFETY: The header fits in storage and the payload starts at offset.
        unsafe { pointer.cast::<Header>().write(Header { epoch }) };
        if epoch != 0 {
            let live = LIVE.fetch_add(layout.size(), SeqCst) + layout.size();
            PEAK.fetch_max(live, SeqCst);
        }
        // SAFETY: The offset is within storage and satisfies the payload alignment.
        unsafe { pointer.add(offset) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let Some((storage, offset)) = storage(layout) else {
            return;
        };
        // SAFETY: The caller supplies the original layout and a live payload pointer.
        let base = unsafe { pointer.sub(offset) };
        // SAFETY: Each allocation carries an initialized header before the payload.
        let header = unsafe { base.cast::<Header>().read() };
        if header.epoch != 0 && header.epoch == ACTIVE.load(SeqCst) {
            LIVE.fetch_sub(layout.size(), SeqCst);
        }
        // SAFETY: Base and storage are the original System allocation.
        unsafe { System.dealloc(base, storage) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let Ok(next) = Layout::from_size_align(size, layout.align()) else {
            return std::ptr::null_mut();
        };
        let Some((old_storage, old_offset)) = storage(layout) else {
            return std::ptr::null_mut();
        };
        let Some((new_storage, new_offset)) = storage(next) else {
            return std::ptr::null_mut();
        };
        debug_assert_eq!(old_offset, new_offset);
        // SAFETY: The caller supplies the original layout and a live payload pointer.
        let base = unsafe { pointer.sub(old_offset) };
        // SAFETY: Each allocation carries an initialized header.
        let header = unsafe { base.cast::<Header>().read() };
        // SAFETY: Both layouts use the same alignment and payload offset.
        let result = unsafe { System.realloc(base, old_storage, new_storage.size()) };
        if result.is_null() {
            return result;
        }
        if header.epoch != 0 && header.epoch == ACTIVE.load(SeqCst) {
            if size > layout.size() {
                let live = LIVE.fetch_add(size - layout.size(), SeqCst) + size - layout.size();
                PEAK.fetch_max(live, SeqCst);
            } else {
                LIVE.fetch_sub(layout.size() - size, SeqCst);
            }
        }
        // SAFETY: The unchanged payload offset fits in the new allocation.
        unsafe { result.add(new_offset) }
    }
}

#[global_allocator]
static ALLOCATOR: Counter = Counter;

pub fn measure<T>(compile: impl FnOnce() -> T) -> (T, usize) {
    // The measurement lock serializes calls. Allocation tags survive transfers between threads.
    LIVE.store(0, SeqCst);
    PEAK.store(0, SeqCst);
    CALLER.with(|caller| caller.set(true));
    ACTIVE.store(EPOCH.fetch_add(1, SeqCst) + 1, SeqCst);
    let result = compile();
    ACTIVE.store(0, SeqCst);
    CALLER.with(|caller| caller.set(false));
    (result, PEAK.load(SeqCst))
}
