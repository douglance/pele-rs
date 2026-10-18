//! Allocation counts only. Never use this instrumented binary for elapsed timings.
#[path = "../workload.rs"]
mod workload;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering::Relaxed};
static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static REALLOCS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED: AtomicU64 = AtomicU64::new(0);
static FREED: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicI64 = AtomicI64::new(0);
static PEAK: AtomicI64 = AtomicI64::new(0);
struct Counted;
fn add(bytes: usize) {
    ALLOCATED.fetch_add(bytes as u64, Relaxed);
    let live = LIVE.fetch_add(bytes as i64, Relaxed) + bytes as i64;
    PEAK.fetch_max(live, Relaxed);
}
fn remove(bytes: usize) {
    FREED.fetch_add(bytes as u64, Relaxed);
    LIVE.fetch_sub(bytes as i64, Relaxed);
}
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() && ENABLED.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            add(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(l);
        if !p.is_null() && ENABLED.load(Relaxed) {
            ALLOCS.fetch_add(1, Relaxed);
            add(l.size());
        }
        p
    }
    unsafe fn realloc(&self, p: *mut u8, old: Layout, size: usize) -> *mut u8 {
        let q = System.realloc(p, old, size);
        if !q.is_null() && ENABLED.load(Relaxed) {
            REALLOCS.fetch_add(1, Relaxed);
            remove(old.size());
            add(size);
        }
        q
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if ENABLED.load(Relaxed) {
            remove(l.size());
        }
        System.dealloc(p, l);
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;
fn capture_allocations(
    work: impl FnOnce() -> Result<u64, String>,
) -> Result<serde_json::Value, String> {
    ALLOCS.store(0, Relaxed);
    REALLOCS.store(0, Relaxed);
    ALLOCATED.store(0, Relaxed);
    FREED.store(0, Relaxed);
    LIVE.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    ENABLED.store(true, Relaxed);
    let result = work();
    ENABLED.store(false, Relaxed);
    Ok(
        serde_json::json!({"checksum":result?,"allocations":ALLOCS.load(Relaxed),"reallocations":REALLOCS.load(Relaxed),
        "requestedBytes":ALLOCATED.load(Relaxed),"freedBytes":FREED.load(Relaxed),
        "liveBytesDelta":LIVE.load(Relaxed),"peakAdditionalRequestedBytes":PEAK.load(Relaxed)}),
    )
}
const ALLOCATION_BUILD: bool = true;
fn main() {
    workload::main();
}
