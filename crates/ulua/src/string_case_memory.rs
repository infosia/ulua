//! Cost: 0.315 s on macOS arm64, Rust 1.95.0 debug.
//! One Unicode scalar scan and conversion samples through 4097 bytes.
//! The scan finds expansion witnesses; samples cover UTF-8 widths and capacity boundaries.
use super::{convert, reservation};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    time::Instant,
};

struct Counter;
thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static LIVE: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
}
fn add(n: usize) {
    LIVE.with(|live| {
        let next = live.get() + n;
        live.set(next);
        PEAK.with(|peak| peak.set(peak.get().max(next)));
    });
}
// SAFETY: System owns each allocation. Reallocation preserves bytes and layout alignment.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller provides a valid layout.
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() && ACTIVE.with(Cell::get) {
            add(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        if ACTIVE.with(Cell::get) {
            LIVE.with(|live| live.set(live.get() - layout.size()));
        }
        // SAFETY: The caller provides the original pointer and layout.
        unsafe { System.dealloc(p, layout) };
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if !ACTIVE.with(Cell::get) {
            // SAFETY: The caller supplies the live pointer, original layout, and new size.
            return unsafe { System.realloc(p, layout, size) };
        }
        let Ok(next) = Layout::from_size_align(size, layout.align()) else {
            return std::ptr::null_mut();
        };
        // Force a moving realloc. Count both allocations before releasing the old allocation.
        // SAFETY: The new layout is valid; the old allocation contains at least min(old, new) bytes.
        unsafe {
            let q = self.alloc(next);
            if !q.is_null() {
                std::ptr::copy_nonoverlapping(p, q, size.min(layout.size()));
                self.dealloc(p, layout);
            }
            q
        }
    }
}
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn measure(op: u32, input: &str) -> usize {
    LIVE.with(|v| v.set(0));
    PEAK.with(|v| v.set(0));
    ACTIVE.with(|v| v.set(true));
    let result = convert(op, 0.0, input);
    std::hint::black_box(&result);
    drop(result);
    ACTIVE.with(|v| v.set(false));
    assert_eq!(LIVE.with(Cell::get), 0);
    PEAK.with(Cell::get)
}
// Growth regimes: n=2,3 reach the eight-byte minimum; n>=4 reaches n+2n (lower) or 2n+4n (upper).
// Lengths through 65 cross these regimes and ASCII chunk boundaries; larger samples reach 4097 bytes.
#[test]
fn conversion_peak() {
    let start = Instant::now();
    let mut witnesses = [Vec::new(), Vec::new()];
    let mut widths = [[(0, '\0'); 4]; 2];
    let mut ratios = [(0, 1); 2];
    for c in (0..=0x10ffff).filter_map(char::from_u32) {
        for (i, op) in [10, 11].into_iter().enumerate() {
            let mut bytes = [0; 4];
            let s = c.encode_utf8(&mut bytes);
            let output = convert(op, 0.0, s).unwrap();
            let order = (output.len() * ratios[i].1).cmp(&(ratios[i].0 * s.len()));
            if order.is_gt() {
                ratios[i] = (output.len(), s.len());
                witnesses[i].clear();
            }
            if order.is_ge() {
                witnesses[i].push(c);
            }
            if output.len() > widths[i][s.len() - 1].0 {
                widths[i][s.len() - 1] = (output.len(), c);
            }
        }
    }
    eprintln!(
        "expansion witnesses: {witnesses:?}, ratios: {ratios:?}, widths: {widths:?}; scan={:.3}s",
        start.elapsed().as_secs_f64()
    );
    let mut lengths: Vec<usize> = (0..=65).collect();
    for power in 7..=12 {
        let n = 1 << power;
        lengths.extend([n - 1, n, n + 1]);
    }
    lengths.sort_unstable();
    lengths.dedup();
    for n in lengths {
        let sample_start = Instant::now();
        let mut peaks = [0; 2];
        for (i, op) in [10, 11].into_iter().enumerate() {
            let mut shapes = vec!['a', 'A', 'é', '界', '𐐀'];
            shapes.extend(&witnesses[i]);
            shapes.extend(widths[i].iter().map(|&(_, c)| c));
            shapes.sort_unstable();
            shapes.dedup();
            for c in shapes {
                for prefix in [0, 1, 7, 15, 16, 17, 31, 63] {
                    if prefix > n {
                        continue;
                    }
                    let mut input = "A".repeat(prefix);
                    let rest = n - prefix;
                    input.push_str(&c.to_string().repeat(rest / c.len_utf8()));
                    input.push_str(&"a".repeat(rest % c.len_utf8()));
                    assert_eq!(input.len(), n);
                    let peak = measure(op, &input);
                    assert!(
                        peak <= reservation(op, n).unwrap(),
                        "op={op}, n={n}, char={c:?}, prefix={prefix}, peak={peak}"
                    );
                    peaks[i] = peaks[i].max(peak);
                }
            }
        }
        eprintln!(
            "{n},{},{},cost={:.6}s",
            peaks[0],
            peaks[1],
            sample_start.elapsed().as_secs_f64()
        );
    }
    eprintln!("conversion peak cost={:.3}s", start.elapsed().as_secs_f64());
}

// Cost: constant arithmetic checks and eight allocation-free dispatch queries.
#[test]
fn reservation_overflow_and_query() {
    for op in [10, 11] {
        assert_eq!(reservation(op, usize::MAX), None);
        assert_eq!(reservation(op, 0), Some(0));
        // SAFETY: Operations 15/16 select the allocation-free query without pointer access.
        unsafe {
            assert_eq!(
                super::dispatch(op + 5, 4.0, std::ptr::null(), 0, std::ptr::null_mut()),
                reservation(op, 4).unwrap() as i32
            );
            let factor = if op == 10 { 6 } else { 3 };
            let limit = i32::MAX as usize / factor;
            assert_eq!(
                super::dispatch(
                    op + 5,
                    limit as f64,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut()
                ),
                (limit * factor) as i32
            );
            assert_eq!(
                super::dispatch(
                    op + 5,
                    (limit + 1) as f64,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut()
                ),
                -1
            );
            assert_eq!(
                super::dispatch(
                    op + 5,
                    i32::MAX as f64,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut()
                ),
                -1
            );
        }
    }
}
