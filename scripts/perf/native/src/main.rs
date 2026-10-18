mod workload;
fn capture_allocations(
    _: impl FnOnce() -> Result<u64, String>,
) -> Result<serde_json::Value, String> {
    Err("Use the separate pele-perf-alloc binary for allocation counts".into())
}
const ALLOCATION_BUILD: bool = false;
fn main() {
    workload::main();
}
