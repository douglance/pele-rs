//! Owned selective built-in registry, independent of its retained documents.
use super::{compatibility_call, compatibility_input, public, PelePublicDocument, PeleResult};
use pele::compat::{public::PublicEngine, DiagramId, DiagramType, PeleError, DIAGRAM_TYPES};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Mutex, MutexGuard, TryLockError},
};

pub struct PeleRegistry {
    engine: Mutex<PublicEngine>,
}
unsafe fn registry<'a>(
    handle: *const PeleRegistry,
) -> Result<MutexGuard<'a, PublicEngine>, PeleError> {
    let owner = unsafe { handle.as_ref() }
        .ok_or_else(|| public::invalid("Native registry transport: null registry"))?;
    owner.engine.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => public::invalid("Native registry is already in use"),
        TryLockError::Poisoned(_) => {
            public::invalid("Native registry is unavailable after an engine panic")
        }
    })
}
fn builtin(id: &str) -> Option<DiagramType> {
    DIAGRAM_TYPES
        .iter()
        .find(|kind| kind.as_str() == id)
        .copied()
}
/// # Safety
/// out_registry must be writable. Every failure writes null; successful ownership
/// is published only after the owned result has been encoded.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_create_v8(
    mode: u32,
    out_registry: *mut *mut PeleRegistry,
) -> *mut PeleResult {
    compatibility_call(|| {
        let out = unsafe { out_registry.as_mut() }
            .ok_or_else(|| public::invalid("Native registry transport: null registry output"))?;
        *out = std::ptr::null_mut();
        let engine = match mode {
            0 => PublicEngine::empty(),
            1 => PublicEngine::with_builtins(),
            _ => {
                return Err(public::invalid(
                    "Native registry transport: unknown creation mode",
                ))
            }
        };
        let result = public::success(true)?;
        *out = Box::into_raw(Box::new(PeleRegistry {
            engine: Mutex::new(engine),
        }));
        Ok(result)
    })
}
/// # Safety
/// Free null or exactly one owned registry after all operations have returned.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_free_v8(handle: *mut PeleRegistry) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !handle.is_null() {
            drop(unsafe { Box::from_raw(handle) });
        }
    }));
}
/// # Safety
/// handle is live or null; do not free concurrently with this call.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_capabilities_v8(
    handle: *const PeleRegistry,
) -> *mut PeleResult {
    compatibility_call(|| public::success(unsafe { registry(handle) }?.capabilities()))
}
/// # Safety
/// id is readable UTF-8 up to4096 bytes, or null with zero length.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_registered_v8(
    handle: *const PeleRegistry,
    id: *const u8,
    len: usize,
) -> *mut PeleResult {
    compatibility_call(|| {
        let name = unsafe { compatibility_input(id, len, 4096) }?;
        let id = builtin(name).map_or_else(|| DiagramId::extension(name), DiagramId::from);
        public::success(unsafe { registry(handle) }?.registered(id))
    })
}
/// # Safety
/// Same ID/handle borrow rules as registered_v8. Only exact built-in IDs register.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_register_builtin_v8(
    handle: *mut PeleRegistry,
    id: *const u8,
    len: usize,
) -> *mut PeleResult {
    compatibility_call(|| {
        let name = unsafe { compatibility_input(id, len, 4096) }?;
        let kind = builtin(name).ok_or_else(|| {
            public::invalid(&format!("Unknown built-in diagram type \"{name}\"."))
        })?;
        unsafe { registry(handle) }?.register_builtin(kind);
        public::success(true)
    })
}
/// # Safety
/// source is readable UTF-8 up to16MiB. No parse threshold/grammar validation.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_detect_type_v8(
    handle: *const PeleRegistry,
    source: *const u8,
    len: usize,
) -> *mut PeleResult {
    compatibility_call(|| {
        let source =
            unsafe { compatibility_input(source, len, pele::compat::MAX_PUBLIC_SOURCE_BYTES) }?;
        let kind = unsafe { registry(handle) }?.detect_type(source)?;
        public::success(serde_json::json!({"type":kind.map(|value| value.as_str())}))
    })
}
/// # Safety
/// Same source borrow/budget as detect_type_v8. Preprocessing failure is false;
/// invalid UTF-8/transport failures remain outer errors.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_supports_v8(
    handle: *const PeleRegistry,
    source: *const u8,
    len: usize,
) -> *mut PeleResult {
    compatibility_call(|| {
        let source =
            unsafe { compatibility_input(source, len, pele::compat::MAX_PUBLIC_SOURCE_BYTES) }?;
        public::success(unsafe { registry(handle) }?.supports(source))
    })
}
/// # Safety
/// Same source/options/output ownership and limits as public_parse_limits_v7.
/// Documents retain registration/config/state independently of registry lifetime.
#[no_mangle]
pub unsafe extern "C" fn pele_registry_parse_limits_v8(
    handle: *const PeleRegistry,
    source: *const u8,
    source_len: usize,
    options: *const u8,
    options_len: usize,
    source_kind: u32,
    source_bits: u64,
    edge_kind: u32,
    edge_bits: u64,
    out_document: *mut *mut PelePublicDocument,
) -> *mut PeleResult {
    compatibility_call(|| {
        let out = unsafe { out_document.as_mut() }
            .ok_or_else(|| public::invalid("Native public transport: null document output"))?;
        *out = std::ptr::null_mut();
        let source = unsafe {
            compatibility_input(source, source_len, pele::compat::MAX_PUBLIC_SOURCE_BYTES)
        }?;
        let options = unsafe { compatibility_input(options, options_len, 1024 * 1024) }?;
        let limits = public::decode_parse_limits(source_kind, source_bits, edge_kind, edge_bits)?;
        let options = public::parse_limits_options(
            options,
            limits.source.is_some(),
            limits.max_edges.is_some(),
        )?;
        let document = unsafe { registry(handle) }?.parse_with_limits(source, &options, limits)?;
        public::publish_document(document, out)
    })
}
