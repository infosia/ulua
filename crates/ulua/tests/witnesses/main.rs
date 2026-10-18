//! Time-ratio assertions share one executable and one lock for each complete test body.
#![allow(clippy::duplicate_mod)]
use std::sync::{Mutex, MutexGuard};

mod callback_growth;
mod compile_memory;
mod p25_growth;
mod p25_witness_time;
mod p3_growth;
mod p41_growth;
mod p55_capture;
mod p55_ratio;
mod p63_performance;
mod p73_number;
mod remainder_growth;
mod s4_growth;

fn measurement_lock() -> MutexGuard<'static, ()> {
    static MEASUREMENT: Mutex<()> = Mutex::new(());
    MEASUREMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}
