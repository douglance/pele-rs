//! Borrowed, bounded access to an active model arena. IDs retain arena identity;
//! implementations update their native indexes rather than copying a model graph.
use super::{Atom, JsString, MethodCall, Node, PublicError};
use crate::compat::RenderResources;
use serde::{Deserialize, Serialize};

/// Append-only string/object identities plus changed records, exchanged only
/// while a host callback owns the active arena. No full graph projection.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelDelta {
    pub strings_start: u32,
    pub strings: Vec<JsString>,
    pub nodes_start: u32,
    pub nodes: Vec<(u32, Node)>,
}
fn unavailable() -> PublicError {
    crate::compat::PeleError::new(
        "Active model synchronization is unavailable",
        crate::compat::ErrorCode::Semantic,
    )
    .into()
}
pub trait ActiveModel {
    /// Opt-in synchronization; ordinary native callbacks allocate no journal.
    fn begin_sync(&mut self) -> Result<(), PublicError> {
        Err(unavailable())
    }
    fn apply_delta(&mut self, _delta: ModelDelta) -> Result<(), PublicError> {
        Err(unavailable())
    }
    fn take_delta(&mut self) -> Result<ModelDelta, PublicError> {
        Err(unavailable())
    }

    fn root(&self) -> Atom;
    fn validate(&self, value: &Atom) -> Result<(), PublicError>;
    fn read(&mut self, receiver: &Atom, key: &str) -> Result<Atom, PublicError>;
    fn set(&mut self, receiver: &Atom, key: &str, value: Atom) -> Result<(), PublicError>;
    fn intern(&mut self, value: JsString) -> Result<Atom, PublicError>;
    fn string(&mut self, value: &Atom) -> Result<JsString, PublicError>;
    fn object(&mut self) -> Result<Atom, PublicError>;
    fn array(&mut self) -> Result<Atom, PublicError>;
    fn date(&mut self, milliseconds: f64) -> Result<Atom, PublicError>;
    fn date_millis(&mut self, value: &Atom) -> Result<f64, PublicError>;
    fn push(&mut self, array: &Atom, value: Atom) -> Result<(), PublicError>;
    /// Dispatch the captured native method, never a replaced own property.
    fn invoke_builtin(
        &mut self,
        call: MethodCall,
        resources: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError>;
}

/// An owner-retained callable. No Send/Sync promise; call and model borrows are
/// synchronous. Resolving the owned callable before invocation lets the original
/// built-in use the same render session without aliasing the host adapter.
pub trait HostFunction {
    fn call(
        &self,
        model: &mut dyn ActiveModel,
        receiver: Atom,
        arguments: &[Atom],
        resources: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError>;
}
